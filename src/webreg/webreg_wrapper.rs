#![allow(dead_code)]

use crate::util::get_epoch_time;
use crate::webreg::webreg_clean_defn::{
    CourseSection, EnrollmentStatus, Meeting, MeetingDay, ScheduledSection,
};
use crate::webreg::webreg_helper;
use crate::webreg::webreg_raw_defn::{ScheduledMeeting, WebRegMeeting, WebRegSearchResultItem};
use reqwest::header::{COOKIE, USER_AGENT};
use reqwest::{Client, Error, Response};
use serde_json::{json, Value};
use std::cmp::max;
use std::collections::{HashMap, HashSet, VecDeque};
use url::Url;

const MY_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
like Gecko) Chrome/97.0.4692.71 Safari/537.36";

const DEFAULT_SCHEDULE_NAME: &str = "My Schedule";

// Random WebReg links
const WEBREG_BASE: &str = "https://act.ucsd.edu/webreg2";
const WEBREG_SEARCH: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-by-all?";
const ACC_NAME: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/get-current-name";
const COURSE_DATA: &str =
    "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-load-group-data?";
const CURR_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/get-class?";
const SEND_EMAIL: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/send-email";
const CHANGE_ENROLL: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/change-enroll";
const PLAN_ADD: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-add";
const PLAN_REMOVE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-remove";
const PLAN_EDIT: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/edit-plan";
const PING_SERVER: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/ping-server";
const REMOVE_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-remove";
const RENAME_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/plan-rename";
const ALL_SCHEDULE: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-get-schednames";

/// A wrapper for [UCSD's WebReg](https://act.ucsd.edu/webreg2/start).
pub struct WebRegWrapper<'a> {
    cookies: &'a str,
    client: Client,
    term: &'a str,
}

impl<'a> WebRegWrapper<'a> {
    /// Creates a new instance of the `WebRegWrapper`.
    ///
    /// # Parameters
    /// - `cookies`: The cookies from your session of WebReg.
    ///
    /// # Returns
    /// The new instance.
    pub fn new(cookies: &'a str, term: &'a str) -> Self {
        WebRegWrapper {
            cookies,
            client: Client::new(),
            term,
        }
    }

    /// Checks if the current WebReg instance is valid.
    ///
    /// # Returns
    /// `true` if the instance is valid and `false` otherwise.
    pub async fn is_valid(&self) -> bool {
        let res = self
            .client
            .get(WEBREG_BASE)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => self._internal_is_valid(&r.text().await.unwrap()),
        }
    }

    /// Gets the name of the owner associated with this account.
    ///
    /// # Returns
    /// The name of the person, or an empty string if the cookies that were given were invalid.
    pub async fn get_account_name(&self) -> String {
        let res = self
            .client
            .get(ACC_NAME)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => "".to_string(),
            Ok(r) => {
                let name = r.text().await.unwrap();
                if self._internal_is_valid(&name) {
                    name
                } else {
                    "".to_string()
                }
            }
        }
    }

    /// Gets your current schedule.
    ///
    /// # Parameters
    /// - `schedule_name`: The schedule that you want to get. If `None` is given, this will default
    /// to your main schedule.
    ///
    /// # Returns
    /// A vector containing the courses that you are enrolled in, or `None` if this isn't possible.
    pub async fn get_schedule(&self, schedule_name: Option<&str>) -> Option<Vec<ScheduledSection>> {
        let url = Url::parse_with_params(
            CURR_SCHEDULE,
            &[
                ("schedname", schedule_name.unwrap_or(DEFAULT_SCHEDULE_NAME)),
                ("final", ""),
                ("sectnum", ""),
                ("termcode", self.term),
            ],
        )
        .unwrap();

        let res = self
            .client
            .get(url)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => None,
            Ok(r) => {
                if !r.status().is_success() {
                    return None;
                }

                let text = r.text().await.unwrap_or_else(|_| "".to_string());
                if text.is_empty() {
                    return None;
                }

                let parsed =
                    serde_json::from_str::<Vec<ScheduledMeeting>>(&text).unwrap_or_default();
                if parsed.is_empty() {
                    return Some(vec![]);
                }

                let mut most_occurring_start_dates: HashMap<&str, u32> = HashMap::new();
                for m in &parsed {
                    *most_occurring_start_dates.entry(&m.start_date).or_insert(0) += 1;
                }

                // Presumably, any lecture or "main" section will have the most common starting
                // date.
                let common_date = most_occurring_start_dates
                    .into_iter()
                    .max_by_key(|&(_, c)| c)
                    .unwrap()
                    .0;

                let mut base_group_secs: HashMap<&str, Vec<&ScheduledMeeting>> = HashMap::new();
                let mut special_classes: HashMap<&str, Vec<&ScheduledMeeting>> = HashMap::new();
                for s_meeting in &parsed {
                    if webreg_helper::is_useless_section(&s_meeting.sect_code) {
                        continue;
                    }

                    if s_meeting.sect_code.as_bytes()[0].is_ascii_digit() {
                        special_classes
                            .entry(s_meeting.course_title.trim())
                            .or_insert_with(Vec::new)
                            .push(s_meeting);

                        continue;
                    }

                    base_group_secs
                        .entry(s_meeting.course_title.trim())
                        .or_insert_with(Vec::new)
                        .push(s_meeting);
                }

                let mut schedule: Vec<ScheduledSection> = vec![];

                for (_, sch_meetings) in base_group_secs {
                    // Literally all just to find the "main" lecture since webreg is inconsistent
                    // plus some courses may not have a lecture.
                    let all_main = sch_meetings
                        .iter()
                        .filter(|x| x.sect_code.ends_with("00") && x.start_date == common_date)
                        .collect::<Vec<_>>();
                    assert!(
                        !all_main.is_empty()
                            && all_main
                                .iter()
                                .all(|x| x.meeting_type == all_main[0].meeting_type)
                    );
                    let day_code = all_main
                        .iter()
                        .map(|x| x.day_code.trim())
                        .collect::<Vec<_>>()
                        .join("");

                    let mut all_meetings: Vec<Meeting> = vec![Meeting {
                        meeting_type: all_main[0].meeting_type.to_string(),
                        meeting_days: if day_code.is_empty() {
                            MeetingDay::None
                        } else {
                            MeetingDay::Repeated(webreg_helper::parse_day_code(&day_code))
                        },
                        start_min: all_main[0].start_time_min,
                        start_hr: all_main[0].start_time_hr,
                        end_min: all_main[0].end_time_min,
                        end_hr: all_main[0].end_time_hr,
                        building: all_main[0].bldg_code.trim().to_string(),
                        room: all_main[0].room_code.trim().to_string(),
                    }];

                    // TODO calculate waitlist somehow
                    // Calculate the remaining meetings. other_special consists of midterms and
                    // final exams, for example, since they are all shared in the same overall
                    // section (e.g. A02 & A03 are in A00)
                    sch_meetings
                        .iter()
                        .filter(|x| x.sect_code.ends_with("00") && x.start_date != common_date)
                        .map(|x| Meeting {
                            meeting_type: x.meeting_type.to_string(),
                            meeting_days: MeetingDay::OneTime(x.start_date.to_string()),
                            start_min: x.start_time_min,
                            start_hr: x.start_time_hr,
                            end_min: x.end_time_min,
                            end_hr: x.end_time_hr,
                            building: x.bldg_code.trim().to_string(),
                            room: x.room_code.trim().to_string(),
                        })
                        .for_each(|meeting| all_meetings.push(meeting));

                    // Other meetings
                    sch_meetings
                        .iter()
                        .filter(|x| !x.sect_code.ends_with("00"))
                        .map(|x| Meeting {
                            meeting_type: x.meeting_type.to_string(),
                            meeting_days: MeetingDay::Repeated(webreg_helper::parse_day_code(
                                &x.day_code,
                            )),
                            start_min: x.start_time_min,
                            start_hr: x.start_time_hr,
                            end_min: x.end_time_min,
                            end_hr: x.end_time_hr,
                            building: x.bldg_code.trim().to_string(),
                            room: x.room_code.trim().to_string(),
                        })
                        .for_each(|meeting| all_meetings.push(meeting));

                    schedule.push(ScheduledSection {
                        section_number: sch_meetings[0].section_number,
                        instructor: sch_meetings[0].person_full_name.trim().to_string(),
                        subject_code: sch_meetings[0].subj_code.trim().to_string(),
                        course_code: sch_meetings[0].course_code.trim().to_string(),
                        course_title: sch_meetings[0].course_title.trim().to_string(),
                        section_code: match sch_meetings
                            .iter()
                            .find(|x| !x.sect_code.ends_with("00"))
                        {
                            Some(r) => r.sect_code.to_string(),
                            None => sch_meetings[0].sect_code.to_string(),
                        },
                        section_capacity: match sch_meetings
                            .iter()
                            .find(|x| x.section_capacity.is_some())
                        {
                            Some(r) => r.section_capacity.unwrap(),
                            None => -1,
                        },
                        enrolled_count: match sch_meetings
                            .iter()
                            .find(|x| x.enrolled_count.is_some())
                        {
                            Some(r) => r.enrolled_count.unwrap(),
                            None => -1,
                        },
                        grade_option: sch_meetings[0].grade_option.trim().to_string(),
                        units: sch_meetings[0].sect_credit_hrs,
                        enrolled_status: match &*sch_meetings[0].enroll_status {
                            "EN" => EnrollmentStatus::Enrolled,
                            "WT" => EnrollmentStatus::Waitlist(-1),
                            "PL" => EnrollmentStatus::Planned,
                            _ => EnrollmentStatus::Planned,
                        },
                        waitlist_ct: -1,
                        meetings: all_meetings,
                    });
                }

                for (_, sch_meetings) in special_classes {
                    let day_code = sch_meetings
                        .iter()
                        .map(|x| x.day_code.trim())
                        .collect::<Vec<_>>()
                        .join("");

                    let parsed_day_code = if day_code.is_empty() {
                        MeetingDay::None
                    } else {
                        MeetingDay::Repeated(webreg_helper::parse_day_code(&day_code))
                    };

                    schedule.push(ScheduledSection {
                        section_number: sch_meetings[0].section_number,
                        instructor: sch_meetings[0].person_full_name.trim().to_string(),
                        subject_code: sch_meetings[0].subj_code.trim().to_string(),
                        course_code: sch_meetings[0].course_code.trim().to_string(),
                        course_title: sch_meetings[0].course_title.trim().to_string(),
                        section_code: sch_meetings[0].sect_code.to_string(),
                        section_capacity: sch_meetings[0].section_capacity.unwrap_or(-1),
                        enrolled_count: sch_meetings[0].enrolled_count.unwrap_or(-1),
                        grade_option: sch_meetings[0].grade_option.trim().to_string(),
                        units: sch_meetings[0].sect_credit_hrs,
                        enrolled_status: match &*sch_meetings[0].enroll_status {
                            "EN" => EnrollmentStatus::Enrolled,
                            "WT" => EnrollmentStatus::Waitlist(-1),
                            "PL" => EnrollmentStatus::Planned,
                            _ => EnrollmentStatus::Planned,
                        },
                        waitlist_ct: -1,
                        meetings: vec![Meeting {
                            meeting_type: sch_meetings[0].meeting_type.to_string(),
                            meeting_days: parsed_day_code,
                            start_min: sch_meetings[0].start_time_min,
                            start_hr: sch_meetings[0].start_time_hr,
                            end_min: sch_meetings[0].end_time_min,
                            end_hr: sch_meetings[0].start_time_hr,
                            building: sch_meetings[0].bldg_code.trim().to_string(),
                            room: sch_meetings[0].room_code.trim().to_string(),
                        }],
                    });
                }

                Some(schedule)
            }
        }
    }

    /// Gets enrollment information on a particular course.
    ///
    /// Note that WebReg provides this information in a way that makes it hard to use; in
    /// particular, WebReg separates each lecture, discussion, final exam, etc. from each other.
    /// This function attempts to figure out which lecture/discussion/final exam/etc. correspond
    /// to which section.
    ///
    /// # Parameters
    /// - `subject_code`: The subject code. For example, if you wanted to check `MATH 100B`, you
    /// would put `MATH`.
    /// - `course_code`: The course code. For example, if you wanted to check `MATH 100B`, you
    /// would put `100B`.
    ///
    /// # Returns
    /// An option containing either:
    /// - A vector with all possible sections that match the given subject code & course code.
    /// - Or nothing.
    pub async fn get_course_info(
        &self,
        subject_code: &str,
        course_code: &str,
    ) -> Option<Vec<CourseSection>> {
        let crsc_code = self._get_formatted_course_code(course_code);
        let url = Url::parse_with_params(
            COURSE_DATA,
            &[
                ("subjcode", subject_code),
                ("crsecode", &*crsc_code),
                ("termcode", self.term),
            ],
        )
        .unwrap();

        let res = self
            .client
            .get(url)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => None,
            Ok(r) => {
                if !r.status().is_success() {
                    return None;
                }

                let text = r.text().await.unwrap_or_else(|_| "".to_string());
                if text.is_empty() {
                    return None;
                }

                let course_dept_id =
                    format!("{} {}", subject_code.trim(), course_code.trim()).to_uppercase();
                let parsed: Vec<WebRegMeeting> = serde_json::from_str(&text).unwrap_or_default();

                // Process any "special" sections
                let mut sections: Vec<CourseSection> = vec![];
                let mut unprocessed_sections: Vec<WebRegMeeting> = vec![];
                for webreg_meeting in parsed {
                    if !webreg_helper::is_valid_meeting(&webreg_meeting) {
                        continue;
                    }

                    // If section code starts with a number then it's probably a special section.
                    if webreg_meeting.sect_code.as_bytes()[0].is_ascii_digit() {
                        let m = webreg_helper::parse_meeting_type_date(&webreg_meeting);

                        sections.push(CourseSection {
                            subj_course_id: course_dept_id.clone(),
                            section_id: webreg_meeting.section_number.trim().to_string(),
                            section_code: webreg_meeting.sect_code.trim().to_string(),
                            instructor: webreg_meeting
                                .person_full_name
                                .split_once(';')
                                .unwrap()
                                .0
                                .trim()
                                .to_string(),
                            // Because it turns out that you can have negative available seats.
                            available_seats: max(webreg_meeting.avail_seat, 0),
                            total_seats: webreg_meeting.section_capacity,
                            waitlist_ct: webreg_meeting.count_on_waitlist,
                            meetings: vec![Meeting {
                                start_hr: webreg_meeting.start_time_hr,
                                start_min: webreg_meeting.start_time_min,
                                end_hr: webreg_meeting.end_time_hr,
                                end_min: webreg_meeting.end_time_min,
                                meeting_type: m.0.to_string(),
                                meeting_days: m.1,
                                building: webreg_meeting.bldg_code.trim().to_string(),
                                room: webreg_meeting.room_code.trim().to_string(),
                            }],
                        });

                        continue;
                    }

                    // If the first char of the section code is a letter and the second char of the
                    // section code is a number that is greater than or equal to 5, this is
                    // probably a special meeting (like tutorial, lab, etc.)
                    //
                    // For now, omit it
                    if webreg_helper::is_useless_section(&webreg_meeting.sect_code) {
                        continue;
                    }

                    unprocessed_sections.push(webreg_meeting);
                }

                if unprocessed_sections.is_empty() {
                    return Some(sections);
                }

                // Process remaining sections
                let mut all_groups: Vec<GroupedSection<WebRegMeeting>> = vec![];
                let mut sec_main_ids = unprocessed_sections
                    .iter()
                    .filter(|x| x.sect_code.ends_with("00"))
                    .map(|x| &*x.sect_code)
                    .collect::<VecDeque<_>>();

                let mut seen: HashSet<&str> = HashSet::new();
                while !sec_main_ids.is_empty() {
                    let main_id = sec_main_ids.pop_front().unwrap();
                    if seen.contains(main_id) {
                        continue;
                    }

                    seen.insert(main_id);
                    let letter = main_id.chars().into_iter().next().unwrap();
                    let idx_of_main = unprocessed_sections
                        .iter()
                        .position(|x| {
                            x.sect_code == main_id
                                && x.special_meeting.replace("TBA", "").trim().is_empty()
                        })
                        .expect("This should not have happened!");

                    let mut group = GroupedSection {
                        main_meeting: &unprocessed_sections[idx_of_main],
                        child_meetings: vec![],
                        other_special_meetings: vec![],
                    };

                    // Want all sections with section code starting with the same letter as what
                    // the main section code is. So, if main_id is A00, we want all sections that
                    // have section code starting with A.
                    unprocessed_sections
                        .iter()
                        .filter(|x| x.sect_code.starts_with(letter))
                        .for_each(|x| {
                            // Don't count this again
                            let special_meeting = x.special_meeting.replace("TBA", "");
                            if x.sect_code == main_id && special_meeting.trim().is_empty() {
                                return;
                            }

                            // Probably a discussion
                            if x.start_date == x.section_start_date
                                && special_meeting.trim().is_empty()
                            {
                                group.child_meetings.push(x);
                                return;
                            }

                            group.other_special_meetings.push(x);
                        });

                    all_groups.push(group);
                }

                // Process each group
                for group in all_groups {
                    let (m_m_type, m_days) =
                        webreg_helper::parse_meeting_type_date(group.main_meeting);

                    let main_meeting = Meeting {
                        meeting_type: m_m_type.to_string(),
                        meeting_days: m_days,
                        building: group.main_meeting.bldg_code.trim().to_string(),
                        room: group.main_meeting.room_code.trim().to_string(),
                        start_hr: group.main_meeting.start_time_hr,
                        start_min: group.main_meeting.start_time_min,
                        end_hr: group.main_meeting.end_time_hr,
                        end_min: group.main_meeting.end_time_min,
                    };

                    let other_meetings = group
                        .other_special_meetings
                        .into_iter()
                        .map(|x| {
                            let (o_m_type, o_days) = webreg_helper::parse_meeting_type_date(x);

                            Meeting {
                                meeting_type: o_m_type.to_string(),
                                meeting_days: o_days,
                                building: x.bldg_code.trim().to_string(),
                                room: x.room_code.trim().to_string(),
                                start_hr: x.start_time_hr,
                                start_min: x.start_time_min,
                                end_hr: x.end_time_hr,
                                end_min: x.end_time_min,
                            }
                        })
                        .collect::<Vec<_>>();

                    // It's possible that there are no discussions, just a lecture
                    if group.child_meetings.is_empty() {
                        let mut all_meetings: Vec<Meeting> = vec![main_meeting.clone()];

                        other_meetings
                            .iter()
                            .for_each(|x| all_meetings.push(x.clone()));

                        sections.push(CourseSection {
                            subj_course_id: course_dept_id.clone(),
                            section_id: group.main_meeting.section_number.trim().to_string(),
                            section_code: group.main_meeting.sect_code.trim().to_string(),
                            instructor: group
                                .main_meeting
                                .person_full_name
                                .split_once(';')
                                .unwrap()
                                .0
                                .trim()
                                .to_string(),
                            available_seats: max(group.main_meeting.avail_seat, 0),
                            total_seats: group.main_meeting.section_capacity,
                            waitlist_ct: group.main_meeting.count_on_waitlist,
                            meetings: all_meetings,
                        });

                        continue;
                    }

                    // Hopefully these are discussions
                    for meeting in group.child_meetings {
                        let (m_type, t_m_dats) = webreg_helper::parse_meeting_type_date(meeting);

                        let mut all_meetings: Vec<Meeting> = vec![
                            main_meeting.clone(),
                            Meeting {
                                meeting_type: m_type.to_string(),
                                meeting_days: t_m_dats,
                                start_min: meeting.start_time_min,
                                start_hr: meeting.start_time_hr,
                                end_min: meeting.end_time_min,
                                end_hr: meeting.end_time_hr,
                                building: meeting.bldg_code.trim().to_string(),
                                room: meeting.room_code.trim().to_string(),
                            },
                        ];
                        other_meetings
                            .iter()
                            .for_each(|x| all_meetings.push(x.clone()));

                        sections.push(CourseSection {
                            subj_course_id: course_dept_id.clone(),
                            section_id: meeting.section_number.trim().to_string(),
                            section_code: meeting.sect_code.trim().to_string(),
                            instructor: meeting
                                .person_full_name
                                .split_once(';')
                                .unwrap()
                                .0
                                .trim()
                                .to_string(),
                            available_seats: max(meeting.avail_seat, 0),
                            total_seats: meeting.section_capacity,
                            waitlist_ct: meeting.count_on_waitlist,
                            meetings: all_meetings,
                        });
                    }
                }

                Some(sections)
            }
        }
    }

    /// Gets all courses that are available. This searches for all courses via Webreg's menu, but
    /// then also searches each course found for specific details. This essentially calls the two
    /// functions `search_courses` and `get_course_info`.
    ///
    /// Note: This function call will make *many* API requests. Thus, searching for many classes
    /// is not recommended as you may get rate-limited.
    ///
    /// # Parameters
    /// - `request_filter`: The request filter.
    ///
    /// # Returns
    /// A vector consisting of all courses that are available, with detailed information.
    pub async fn search_courses_detailed(
        &self,
        request_filter: SearchRequestBuilder<'a>,
    ) -> Option<Vec<CourseSection>> {
        let search_res = match self.search_courses(&request_filter).await {
            Some(r) => r,
            None => return None,
        };

        let mut vec: Vec<CourseSection> = vec![];
        for r in search_res {
            let req_res = self
                .get_course_info(r.subj_code.trim(), r.course_code.trim())
                .await;
            match req_res {
                Some(r) => r.into_iter().for_each(|x| vec.push(x)),
                None => break,
            };
        }

        Some(vec)
    }

    /// Gets all courses that are available. All this does is searches for all courses via Webreg's
    /// menu. Thus, only basic details are shown.
    ///
    /// # Parameters
    /// - `request_filter`: The request filter.
    ///
    /// # Returns
    /// A vector consisting of all courses that are available.
    pub async fn search_courses(
        &self,
        request_filter: &SearchRequestBuilder<'a>,
    ) -> Option<Vec<WebRegSearchResultItem>> {
        let subject_code = if request_filter.subjects.is_empty() {
            "".to_string()
        } else {
            request_filter.subjects.join(":")
        };

        let course_code = if request_filter.courses.is_empty() {
            "".to_string()
        } else {
            // This can probably be made significantly more efficient
            request_filter
                .courses
                .iter()
                .map(|x| x.split_whitespace().collect::<Vec<_>>())
                .map(|course| {
                    course
                        .into_iter()
                        .map(|x| self._get_formatted_course_code(x))
                        .collect::<Vec<_>>()
                        .join(":")
                })
                .collect::<Vec<_>>()
                .join(";")
                .to_uppercase()
        };

        let department = if request_filter.departments.is_empty() {
            "".to_string()
        } else {
            request_filter.departments.join(":")
        };

        let professor = match request_filter.instructor {
            Some(r) => r.to_uppercase(),
            None => "".to_string(),
        };

        let title = match request_filter.title {
            Some(r) => r.to_uppercase(),
            None => "".to_string(),
        };

        let levels = if request_filter.level_filter == 0 {
            "".to_string()
        } else {
            // Needs to be exactly 12 digits
            let mut s = format!("{:b}", request_filter.level_filter);
            while s.len() < 12 {
                s.insert(0, '0');
            }

            s
        };

        let days = if request_filter.days == 0 {
            "".to_string()
        } else {
            // Needs to be exactly 7 digits
            let mut s = format!("{:b}", request_filter.days);
            while s.len() < 7 {
                s.insert(0, '0');
            }

            s
        };

        let time_str = {
            if request_filter.start_time.is_none() && request_filter.end_time.is_none() {
                "".to_string()
            } else {
                let start_time = match request_filter.start_time {
                    Some((h, m)) => format!("{:0>2}{:0>2}", h, m),
                    None => "".to_string(),
                };

                let end_time = match request_filter.end_time {
                    Some((h, m)) => format!("{:0>2}{:0>2}", h, m),
                    None => "".to_string(),
                };

                format!("{}:{}", start_time, end_time)
            }
        };

        let url = Url::parse_with_params(
            WEBREG_SEARCH,
            &[
                ("subjcode", &*subject_code),
                ("crsecode", &*course_code),
                ("department", &*department),
                ("professor", &*professor),
                ("title", &*title),
                ("levels", &*levels),
                ("days", &*days),
                ("timestr", &*time_str),
                (
                    "opensection",
                    if request_filter.only_open {
                        "true"
                    } else {
                        "false"
                    },
                ),
                ("isbasic", "true"),
                ("basicsearchvalue", ""),
                ("termcode", self.term),
            ],
        )
        .unwrap();

        let res = self
            .client
            .get(url)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => None,
            Ok(r) => {
                if !r.status().is_success() {
                    return None;
                }

                let text = r.text().await;
                match text {
                    Err(_) => None,
                    Ok(t) => Some(serde_json::from_str(&t).unwrap_or_default()),
                }
            }
        }
    }

    /// Sends an email to yourself using the same email that is used to confirm that you have
    /// enrolled or waitlisted in a particular class. In other words, this will send an email
    /// to you through the email NoReplyRegistrar@ucsd.edu.
    ///
    /// It is strongly recommended that this function not be abused.
    ///
    /// # Parameters
    /// - `email_content`: The email to send.
    ///
    /// # Returns
    /// `true` if the email was sent successfully and `false` otherwise.
    pub async fn send_email_to_self(&self, email_content: &str) -> bool {
        let params: HashMap<&str, &str> =
            HashMap::from([("actionevent", email_content), ("termcode", self.term)]);

        let res = self
            .client
            .post(SEND_EMAIL)
            .form(&params)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => {
                if !r.status().is_success() {
                    false
                } else {
                    r.text().await.unwrap().contains("\"YES\"")
                }
            }
        }
    }

    /// Changes the grading option for the class corresponding to the section number.
    ///
    /// # Parameters
    /// - `section_number`: The section number corresponding to the class that you want to change
    /// the grading option for.
    /// - `new_grade_opt`: The new grading option. This must either be `L` (letter),
    /// `P` (pass/no pass), or `S` (satisfactory/unsatisfactory).
    ///
    /// # Returns
    /// `true` if the process succeeded or `false` otherwise.
    pub async fn change_grading_option(&self, section_number: i64, new_grade_opt: &str) -> bool {
        match new_grade_opt {
            "L" | "P" | "S" => {}
            _ => return false,
        };

        let poss_class = self
            .get_schedule(None)
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|x| x.section_number == section_number);

        if poss_class.is_none() {
            return false;
        }

        // don't care about previous poss_class
        let poss_class = poss_class.unwrap();
        let sec_id = poss_class.section_number.to_string();
        let units = poss_class.units.to_string();

        let params: HashMap<&str, &str> = HashMap::from([
            ("section", &*sec_id),
            ("subjCode", ""),
            ("crseCode", ""),
            ("unit", &*units),
            ("grade", new_grade_opt),
            // You don't actually need these
            ("oldGrade", ""),
            ("oldUnit", ""),
            ("termcode", self.term),
        ]);

        self._process_response(
            self.client
                .post(CHANGE_ENROLL)
                .form(&params)
                .header(COOKIE, self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Allows you to plan a course.
    ///
    /// # Parameters
    /// - `plan_options`: Information for the course that you want to plan.
    /// - `validate`: Whether to validate your planning of this course beforehand.
    /// **WARNING:** setting this to `false` can cause issues. For example, when this is `false`,
    /// you will be able to plan courses with more units than allowed (e.g. 42 units), set the
    /// grading option to one that you are not allowed to use (e.g. S/U as an undergraduate), and
    /// only enroll in specific components of a section (e.g. just the discussion section). Some of
    /// these options can visually break WebReg (e.g. Remove/Enroll button will not appear).
    ///
    /// # Returns
    /// `true` if the course was planned successfully and `false` otherwise.
    pub async fn add_to_plan(&self, plan_options: PlanAdd<'_>, validate: bool) -> bool {
        let u = plan_options.unit_count.to_string();
        let crsc_code = self._get_formatted_course_code(plan_options.course_code);

        if validate {
            // We need to call the edit endpoint first, or else we'll have issues where we don't
            // actually enroll in every component of the course.
            let params_edit: HashMap<&str, &str> = HashMap::from([
                ("section", &*plan_options.section_number),
                ("subjcode", &*plan_options.subject_code),
                ("crsecode", &*crsc_code),
                ("termcode", self.term),
            ]);

            // This can potentially return "false" due to you not being able to enroll in the
            // class, e.g. the class you're trying to plan is a major-restricted class.
            self._process_response(
                self.client
                    .post(PLAN_EDIT)
                    .form(&params_edit)
                    .header(COOKIE, self.cookies)
                    .header(USER_AGENT, MY_USER_AGENT)
                    .send()
                    .await,
            )
            .await;
        }

        let params_add: HashMap<&str, &str> = HashMap::from([
            ("subjcode", &*plan_options.subject_code),
            ("crsecode", &*crsc_code),
            ("sectnum", &*plan_options.section_number),
            ("sectcode", &*plan_options.section_code),
            ("unit", &*u),
            (
                "grade",
                match plan_options.grading_option {
                    Some(r) if r == "L" || r == "P" || r == "S" => r,
                    _ => "L",
                },
            ),
            ("termcode", self.term),
            (
                "schedname",
                match plan_options.schedule_name {
                    Some(r) => r,
                    None => DEFAULT_SCHEDULE_NAME,
                },
            ),
        ]);

        self._process_response(
            self.client
                .post(PLAN_ADD)
                .form(&params_add)
                .header(COOKIE, self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Allows you to unplan a course.
    ///
    /// # Parameters
    /// - `section_num`: The section number.
    /// - `schedule_name`: The schedule name where the course should be unplanned from.
    ///
    /// # Returns
    /// `true` if the course was unplanned successfully and `false` otherwise.
    pub async fn remove_from_plan(
        &self,
        section_num: &str,
        schedule_name: Option<&'a str>,
    ) -> bool {
        let params: HashMap<&str, &str> = HashMap::from([
            ("sectnum", section_num),
            ("termcode", self.term),
            ("schedname", schedule_name.unwrap_or(DEFAULT_SCHEDULE_NAME)),
        ]);

        self._process_response(
            self.client
                .post(PLAN_REMOVE)
                .form(&params)
                .header(COOKIE, self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Pings the WebReg server. Presumably, this is the endpoint that is used to ensure that
    /// your (authenticated) session is still valid. In other words, if this isn't called, I
    /// assume that you will be logged out, rendering your cookies invalid.
    ///
    /// # Returns
    /// `true` if the ping was successful and `false` otherwise.
    pub async fn ping_server(&self) -> bool {
        let res = self
            .client
            .get(format!("{}?_={}", PING_SERVER, get_epoch_time()))
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => {
                let text = r.text().await.unwrap_or_else(|_| {
                    json!({
                        "SESSION_OK": false
                    })
                    .to_string()
                });

                // TODO randomly crashed here, need to fix.
                let json: Value = serde_json::from_str(&text).unwrap_or_default();
                json["SESSION_OK"].is_boolean() && json["SESSION_OK"].as_bool().unwrap()
            }
        }
    }

    /// Renames a schedule to the specified name. You cannot rename the default
    /// `My Schedule` schedule.
    ///
    /// # Parameter
    /// - `old_name`: The name of the old schedule.
    /// - `new_name`: The name that you want to change the old name to.
    ///
    /// # Returns
    /// `true` if the renaming was successful and `false` otherwise.
    pub async fn rename_schedule(&self, old_name: &str, new_name: &str) -> bool {
        // Can't rename your default schedule.
        if old_name == DEFAULT_SCHEDULE_NAME {
            return false;
        }

        let params: HashMap<&str, &str> = HashMap::from([
            ("termcode", self.term),
            ("oldschedname", old_name),
            ("newschedname", new_name),
        ]);

        self._process_response(
            self.client
                .post(RENAME_SCHEDULE)
                .form(&params)
                .header(COOKIE, self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Removes a schedule. You cannot delete the default `My Schedule` one.
    ///
    /// # Parameter
    /// - `schedule_name`: The name of the schedule to delete.
    ///
    /// # Returns
    /// `true` if the deletion was successful and `false` otherwise.
    pub async fn remove_schedule(&self, schedule_name: &str) -> bool {
        // Can't remove your default schedule.
        if schedule_name == DEFAULT_SCHEDULE_NAME {
            return false;
        }

        let params: HashMap<&str, &str> =
            HashMap::from([("termcode", self.term), ("schedname", schedule_name)]);

        self._process_response(
            self.client
                .post(REMOVE_SCHEDULE)
                .form(&params)
                .header(COOKIE, self.cookies)
                .header(USER_AGENT, MY_USER_AGENT)
                .send()
                .await,
        )
        .await
    }

    /// Gets all of your schedules.
    ///
    /// # Returns
    /// A vector of strings representing the names of the schedules, or `None` if
    /// something went wrong.
    pub async fn get_schedules(&self) -> Option<Vec<String>> {
        let url = Url::parse_with_params(ALL_SCHEDULE, &[("termcode", self.term)]).unwrap();

        let res = self
            .client
            .get(url)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => None,
            Ok(r) => {
                if !r.status().is_success() {
                    return None;
                }

                let text = r.text().await;
                match text {
                    Err(_) => None,
                    Ok(t) => Some(serde_json::from_str(&t).unwrap_or_default()),
                }
            }
        }
    }

    async fn _process_response(&self, res: Result<Response, Error>) -> bool {
        match res {
            Err(_) => false,
            Ok(r) => {
                if !r.status().is_success() {
                    false
                } else {
                    let text = r.text().await.unwrap_or_else(|_| {
                        json!({
                            "OPS": "FAIL"
                        })
                        .to_string()
                    });

                    let json: Value = serde_json::from_str(&text).unwrap();
                    json["OPS"].is_string() && json["OPS"].as_str().unwrap() == "SUCCESS"
                }
            }
        }
    }

    /// Gets the current term.
    ///
    /// # Returns
    /// The current term.
    pub fn get_term(&self) -> &'a str {
        self.term
    }

    #[inline(always)]
    fn _internal_is_valid(&self, str: &str) -> bool {
        !str.contains("Skip to main content")
    }

    #[inline(always)]
    fn _get_formatted_course_code(&self, course_code: &str) -> String {
        // If the course code only has 1 digit (excluding any letters), then we need to prepend 2
        // spaces to the course code.
        //
        // If the course code has 2 digits (excluding any letters), then we need to prepend 1
        // space to the course code.
        //
        // Otherwise, don't need to prepend any spaces to the course code.
        //
        // For now, assume that no digits will ever appear *after* the letters. Weird thing is that
        // WebReg uses '+' to offset the course code but spaces are accepted.
        match course_code.chars().filter(|x| x.is_ascii_digit()).count() {
            1 => format!("  {}", course_code),
            2 => format!(" {}", course_code),
            _ => course_code.to_string(),
        }
    }
}

// Helper structure for organizing meetings. Only used once for now.
#[derive(Debug)]
struct GroupedSection<'a, T> {
    main_meeting: &'a T,
    child_meetings: Vec<&'a T>,
    other_special_meetings: Vec<&'a T>,
}

pub struct PlanAdd<'a> {
    /// The subject code. For example, `CSE`.
    pub subject_code: &'a str,
    /// The course code. For example, `12`.
    pub course_code: &'a str,
    /// The section number. For example, `0123123`.
    pub section_number: &'a str,
    /// The section code. For example `A00`.
    pub section_code: &'a str,
    /// The grading option. Can either be L, P, or S.
    pub grading_option: Option<&'a str>,
    /// The schedule name.
    pub schedule_name: Option<&'a str>,
    /// The number of units.
    pub unit_count: u8,
}

/// Used to construct search requests for the `search_courses` function.
pub struct SearchRequestBuilder<'a> {
    subjects: Vec<&'a str>,
    courses: Vec<&'a str>,
    departments: Vec<&'a str>,
    instructor: Option<&'a str>,
    title: Option<&'a str>,
    level_filter: u32,
    days: u32,
    start_time: Option<(u32, u32)>,
    end_time: Option<(u32, u32)>,
    only_open: bool,
}

impl<'a> SearchRequestBuilder<'a> {
    /// Creates a new instance of the `SearchRequestBuilder`, which is used to search for specific
    /// courses.
    ///
    /// # Returns
    /// The empty `SearchRequestBuilder`.
    pub fn new() -> Self {
        Self {
            subjects: vec![],
            courses: vec![],
            departments: vec![],
            instructor: None,
            title: None,
            level_filter: 0,
            days: 0,
            start_time: None,
            end_time: None,
            only_open: false,
        }
    }

    /// Adds a subject to this search request. Valid search requests are uppercase and at most
    /// 4 characters long. Some examples include `MATH` or `CSE`.
    ///
    /// # Parameters
    /// - `subject`: The subject.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn add_subject(mut self, subject: &'a str) -> Self {
        if subject != subject.to_uppercase() || subject.len() > 4 {
            return self;
        }

        self.subjects.push(subject);
        self
    }

    // TODO need to append '+' to course as needed
    /// Adds a course (either a subject code, course code, or both) to the search request. Some
    /// examples include `20E`, `math 20d`, `101`, `CSE`.
    ///
    /// # Parameters
    /// - `course`: The course.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn add_course(mut self, course: &'a str) -> Self {
        self.courses.push(course);
        self
    }

    /// Adds a department to the search request. Valid search requests are uppercase and at most 4
    /// characters long. Some examples include `MATH` or `CSE`.
    ///
    /// # Parameters
    /// - `department`: The department.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn add_department(mut self, department: &'a str) -> Self {
        if department != department.to_uppercase() || department.len() > 4 {
            return self;
        }

        self.departments.push(department);
        self
    }

    /// Sets the instructor to the specified instructor.
    ///
    /// # Parameters
    /// - `instructor`: The instructor. This should be formatted in `Last Name, First Name` form.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_instructor(mut self, instructor: &'a str) -> Self {
        self.instructor = Some(instructor);
        self
    }

    /// Sets the course title to the specified title. Some examples could be `differential equ`,
    /// `data structures`, `algorithms`, and so on.
    ///
    /// # Parameters
    /// - `title`: The title of the course.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Restrict search results to to the specified filter. This can be applied multiple times.
    ///
    /// # Parameters
    /// - `filter`: The filter.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn filter_courses_by(mut self, filter: CourseLevelFilter) -> Self {
        self.level_filter |= match filter {
            CourseLevelFilter::LowerDivision => 1 << 11,
            CourseLevelFilter::FreshmenSeminar => 1 << 10,
            CourseLevelFilter::LowerDivisionIndepStudy => 1 << 9,
            CourseLevelFilter::UpperDivision => 1 << 8,
            CourseLevelFilter::Apprenticeship => 1 << 7,
            CourseLevelFilter::UpperDivisionIndepStudy => 1 << 6,
            CourseLevelFilter::Graduate => 1 << 5,
            CourseLevelFilter::GraduateIndepStudy => 1 << 4,
            CourseLevelFilter::GraduateResearch => 1 << 3,
            CourseLevelFilter::Lvl300 => 1 << 2,
            CourseLevelFilter::Lvl400 => 1 << 1,
            CourseLevelFilter::Lvl500 => 1 << 0,
        };

        self
    }

    /// Only shows courses based on the specified day(s).
    ///
    /// # Parameters
    /// - `day`: The day. Here:
    ///     - Monday is represented as `1`
    ///     - Tuesday is represented as `2`
    ///     - ...
    ///     - Saturday is represented as `6`
    ///     - Sunday is represented as `7`.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn apply_days(mut self, day: u32) -> Self {
        if !(1..=7).contains(&day) {
            return self;
        }

        // Monday = 1
        // Tuesday = 2
        // ...
        // Sunday = 7
        self.days |= 1 << (7 - day);
        self
    }

    /// Sets the start time to the specified time.
    ///
    /// # Parameters
    /// - `hour`: The hour. This should be between 0 and 23, inclusive.
    /// - `min`: The minute. This should be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_start_time(mut self, hour: u32, min: u32) -> Self {
        if hour > 23 || min > 59 {
            return self;
        }

        self.start_time = Some((hour, min));
        self
    }

    /// Sets the end time to the specified time.
    ///
    /// # Parameters
    /// - `hour`: The hour. This should be between 0 and 23, inclusive.
    /// - `min`: The minute. This should be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn set_end_time(mut self, hour: u32, min: u32) -> Self {
        if hour > 23 || min > 59 {
            return self;
        }

        self.end_time = Some((hour, min));
        self
    }

    /// Whether to only show sections with open seats.
    ///
    /// # Returns
    /// The `SearchRequestBuilder`
    pub fn only_allow_open(mut self) -> Self {
        self.only_open = true;
        self
    }
}

pub enum CourseLevelFilter {
    /// Level 1-99 courses.
    LowerDivision,
    /// Level 87, 90 courses.
    FreshmenSeminar,
    /// Level 99 courses.
    LowerDivisionIndepStudy,
    /// Level 100-198 courses
    UpperDivision,
    /// Level 195 courses
    Apprenticeship,
    /// Level 199 courses
    UpperDivisionIndepStudy,
    /// Level 200-297 courses
    Graduate,
    /// Level 298 courses
    GraduateIndepStudy,
    /// Level 299 courses
    GraduateResearch,
    /// Level 300+ courses
    Lvl300,
    /// Level 400+ courses
    Lvl400,
    /// Level 500+ courses
    Lvl500,
}
