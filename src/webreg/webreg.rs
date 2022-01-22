#![allow(dead_code)]

use crate::webreg::webreg_clean_defn::{CourseSection, Meeting};
use crate::webreg::webreg_helper;
use crate::webreg::webreg_raw_defn::{WebRegMeeting, WebRegSearchResultItem};
use reqwest::header::{COOKIE, USER_AGENT};
use reqwest::Client;
use std::cmp::max;
use std::collections::{HashSet, VecDeque};
use url::Url;

const WEBREG_SEARCH: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-by-all?";
const MY_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
like Gecko) Chrome/97.0.4692.71 Safari/537.36";
const WEBREG_NAME_URL: &str = "https://act.ucsd.edu/webreg2/svc/wradapter/get-current-name";
const COURSE_DATA: &str =
    "https://act.ucsd.edu/webreg2/svc/wradapter/secure/search-load-group-data?";

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

    /// Checks if the current WebReg instance is valid. Doesn't actually work.
    ///
    /// # Returns
    /// `true` if the instance is valid and `false` otherwise.
    pub async fn is_valid(&self) -> bool {
        let res = self
            .client
            .get(WEBREG_NAME_URL)
            .header(COOKIE, self.cookies)
            .header(USER_AGENT, MY_USER_AGENT)
            .send()
            .await;

        match res {
            Err(_) => false,
            Ok(r) => r.status().is_success(),
        }
    }

    /// Gets enrollment information on a particular course.
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

        let crsc_code = match course_code.chars().filter(|x| x.is_ascii_digit()).count() {
            1 => format!("  {}", course_code),
            2 => format!(" {}", course_code),
            _ => course_code.to_string(),
        };

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

                let text = r.text().await.unwrap_or("".to_string());
                if text.is_empty() {
                    return None;
                }

                let parsed: Vec<WebRegMeeting> = serde_json::from_str(&text).unwrap_or(vec![]);

                // Process any "special" sections
                let mut sections: Vec<CourseSection> = vec![];
                let mut unprocessed_sections: Vec<WebRegMeeting> = vec![];
                for webreg_meeting in parsed {
                    if !webreg_helper::is_valid_meeting(&webreg_meeting) {
                        continue;
                    }

                    let sec_code = webreg_meeting.sect_code.as_bytes();

                    // If section code starts with a number then it's probably a special section.
                    if sec_code[0].is_ascii_digit() {
                        let m = webreg_helper::parse_meeting_type_date(&webreg_meeting);

                        sections.push(CourseSection {
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
                    if sec_code[0].is_ascii_alphabetic()
                        && sec_code[1].is_ascii_digit()
                        && sec_code[1] as i32 - 48 >= 5
                    {
                        continue;
                    }

                    unprocessed_sections.push(webreg_meeting);
                }

                if unprocessed_sections.is_empty() {
                    return Some(sections);
                }

                // Process remaining sections
                struct GroupedSection<'a> {
                    main_meeting: &'a WebRegMeeting,
                    child_meetings: Vec<&'a WebRegMeeting>,
                    other_special_meetings: Vec<&'a WebRegMeeting>,
                }

                let mut all_groups: Vec<GroupedSection> = vec![];
                let mut sec_main_ids = unprocessed_sections
                    .iter()
                    .filter(|x| x.sect_code.ends_with("00"))
                    .map(|x| &*x.sect_code)
                    .collect::<VecDeque<_>>();

                assert!(!sec_main_ids.is_empty());

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
                        .position(|x| x.sect_code == main_id && x.special_meeting.trim().is_empty())
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
                            if x.sect_code == main_id && x.special_meeting.trim().is_empty() {
                                return;
                            }

                            let special_meeting = x.special_meeting.trim();

                            // Probably a discussion
                            if x.start_date == x.section_start_date && special_meeting.is_empty() {
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
                        webreg_helper::parse_meeting_type_date(&group.main_meeting);

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

    /// Gets all courses that are available. All this does is searches for all courses via Webreg's
    /// menu. Thus, only basic details are shown.
    ///
    /// # Parameters
    /// - `only_open`: Whether to only show open courses.
    ///
    /// # Returns
    /// A vector consisting of all courses that are available.
    pub async fn get_all_courses(&self, only_open: bool) -> Option<Vec<WebRegSearchResultItem>> {
        let url = Url::parse_with_params(
            WEBREG_SEARCH,
            &[
                ("subjcode", ""),
                ("crsecode", ""),
                ("department", ""),
                ("professor", ""),
                ("title", ""),
                ("levels", ""),
                ("days", ""),
                ("timestr", ""),
                ("opensection", if only_open { "true" } else { "false" }),
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
                    Ok(t) => Some(serde_json::from_str(&t).unwrap_or(vec![])),
                }
            }
        }
    }
}
