use std::collections::{HashMap, HashSet};

use crate::webreg::webreg_clean_defn::{CourseSection, MeetingDay};

use super::helper;

pub type Time = (i16, i16);

#[derive(Clone)]
pub struct Schedule<'a> {
    /// All relevant sections.
    pub sections: HashMap<&'a str, &'a CourseSection>,
    /// All used times. This can either be one of Sun, M, ..., F, Sa or
    /// a specified day (e.g. 2022-02-02).
    used_times: HashMap<&'a str, HashSet<(Time, Time)>>,
}

impl<'a> Schedule<'a> {
    /// Creates a new `Schedule`.
    /// 
    /// # Returns
    /// The new `Schedule`. 
    pub fn new() -> Self {
        Schedule {
            sections: HashMap::new(),
            used_times: HashMap::new(),
        }
    }

    /// Checks if the given `CourseSection` can be added.
    /// 
    /// # Parameters
    /// - `course`: The course to check.
    /// 
    /// # Returns
    /// `true` if this can be added and `false` otherwise. 
    pub fn can_add_course(&self, course: &CourseSection) -> bool {
        if self.sections.contains_key(&course.subj_course_id.as_str()) {
            return false;
        }

        for meeting in &course.meetings {
            let new_from_time = (meeting.start_hr, meeting.start_min);
            let new_to_time = (meeting.end_hr, meeting.end_min);

            match meeting.meeting_days {
                MeetingDay::Repeated(ref days) => {
                    for day in days {
                        match self.used_times.get(&day.as_str()) {
                            Some(times) => {
                                for (from_time, to_time) in times {
                                    if helper::time_conflicts(
                                        new_from_time,
                                        new_to_time,
                                        *from_time,
                                        *to_time,
                                    ) {
                                        return false;
                                    }
                                }
                            }
                            None => continue,
                        }
                    }
                },
                MeetingDay::OneTime(ref day) => {
                    match self.used_times.get(&day.as_str()) {
                        Some(times) => {
                            for (from_time, to_time) in times {
                                if helper::time_conflicts(
                                    new_from_time,
                                    new_to_time,
                                    *from_time,
                                    *to_time,
                                ) {
                                    return false;
                                }
                            }
                        }
                        None => continue 
                    }
                },
                MeetingDay::None => todo!(),
            }
        }

        true
    }

    /// Adds the `CourseSection` to the `Schedule`. This assumes that `can_add_course`
    /// has been called.
    /// 
    /// # Parameters
    /// - `course`: The course to add.
    pub fn add_course(&mut self, course: &'a CourseSection) {
        self.sections.insert(course.subj_course_id.as_str(), course); 
        for meeting in &course.meetings {
            let end_time = (meeting.end_hr, meeting.end_min);
            let start_time = (meeting.start_hr, meeting.start_min);

            match meeting.meeting_days {
                MeetingDay::Repeated(ref days) => {
                    for day in days {
                        self.used_times.entry(day.as_str()).or_default().insert((start_time, end_time));
                    }
                },
                MeetingDay::OneTime(ref o) => {
                    self.used_times.entry(o.as_str()).or_default().insert((start_time, end_time));
                },
                MeetingDay::None => continue,
            }
        }
    }
}

/// Generates all possible schedules. This uses a very naive implementation.
///
/// # Parameters
/// - `wanted_courses`: The desired courses.
/// - `all_courses`: All courses to consider.
///
/// # Returns
/// A vector containing all schedules.
pub fn generate_schedules<'a>(
    wanted_courses: &[&str],
    all_courses: &'a [CourseSection],
) -> Vec<Schedule<'a>> {
    // Step 1: Categorize all courses.
    let mut map: HashMap<&str, Vec<&CourseSection>> = HashMap::new();
    for course in all_courses {
        if !wanted_courses.contains(&course.subj_course_id.as_str()) {
            continue;
        }

        map.entry(&course.subj_course_id).or_default().push(course);
    }

    let mut all_schedules: Vec<Schedule<'a>> = vec![];

    if wanted_courses.len() != map.len() {
        return all_schedules;
    }

    let mut curr_schedules: Vec<Schedule<'a>> = vec![];
    let mut added = false;
    'outer: for desired_course in wanted_courses {
        match map.get(desired_course) {
            Some(all_courses) => {
                // Schedule empty means we add initial cases. 
                if curr_schedules.is_empty() {
                    if added {
                        break 'outer;
                    }

                    added = true; 
                    for course in all_courses {
                        let mut s = Schedule::new();
                        s.add_course(course);
                    }

                    continue; 
                }

                let mut sch_to_add: Vec<Schedule<'a>> = vec![];
                for course in all_courses {
                    for temp_schedule in &curr_schedules {
                        if !temp_schedule.can_add_course(&course) {
                            continue;
                        }

                        let mut sch = temp_schedule.clone();
                        sch.add_course(course);
                        sch_to_add.push(sch);
                    }        
                }

                curr_schedules = sch_to_add;
            }
            None => break,
        };
    }

    for schedule in curr_schedules {
        if schedule.sections.len() != wanted_courses.len() {
            continue;
        }

        all_schedules.push(schedule);
    }

    all_schedules
}
