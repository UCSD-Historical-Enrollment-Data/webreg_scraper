use super::helper;
use crate::webreg::webreg_clean_defn::{CourseSection, MeetingDay};
use std::collections::{HashMap, HashSet};

const DAY_OF_WEEK: [&str; 7] = ["Su", "M", "Tu", "W", "Th", "F", "Sa"];

pub type Time = (i16, i16);

#[derive(Clone, Debug)]
pub struct Schedule<'a> {
    /// All relevant sections.
    pub sections: Vec<&'a CourseSection>,
    /// All seen courses.
    pub seen: HashSet<&'a str>,
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
            sections: vec![],
            seen: HashSet::new(),
            used_times: HashMap::new(),
        }
    }

    /// Checks if the given `CourseSection` can be added.
    ///
    /// # Parameters
    /// - `course`: The course to check.
    /// - `constraints`: The constraints.
    ///
    /// # Returns
    /// `true` if this can be added and `false` otherwise.
    pub fn can_add_course(&self, course: &CourseSection, constraints: &ScheduleConstraint) -> bool {
        if self.seen.contains(&course.subj_course_id.as_str()) {
            return false;
        }

        let buffer_offset = match constraints.buffer_time {
            Some(r) => r / 2,
            None => 0,
        };

        let start_time = match constraints.earliest_start {
            Some((h, m)) => h * 100 + m,
            None => 0,
        };

        let end_time = match constraints.latest_end {
            Some((h, m)) => h * 100 + m,
            None => 2359,
        };

        for meeting in &course.meetings {
            let new_from_time = (meeting.start_hr, meeting.start_min);
            let new_from_time_full = meeting.start_hr * 100 + meeting.start_min;
            let new_to_time = (meeting.end_hr, meeting.end_min);
            let new_to_time_full = meeting.end_hr * 100 + meeting.end_min;

            match meeting.meeting_days {
                MeetingDay::Repeated(ref days) => {
                    if new_from_time_full < start_time || new_to_time_full > end_time {
                        return false;
                    }

                    for day in days {
                        if constraints.off_times.iter().any(|(d, a, b)| {
                            d == day && helper::time_conflicts(new_from_time, new_to_time, *a, *b)
                        }) {
                            return false;
                        }

                        match self.used_times.get(&day.as_str()) {
                            Some(times) => {
                                for (from_time, to_time) in times {
                                    if helper::time_conflicts(
                                        helper::calculate_time_with_offset(
                                            new_from_time,
                                            -buffer_offset,
                                        ),
                                        helper::calculate_time_with_offset(
                                            new_to_time,
                                            buffer_offset,
                                        ),
                                        helper::calculate_time_with_offset(
                                            *from_time,
                                            -buffer_offset,
                                        ),
                                        helper::calculate_time_with_offset(*to_time, buffer_offset),
                                    ) {
                                        return false;
                                    }
                                }
                            }
                            None => continue,
                        }
                    }
                }
                MeetingDay::OneTime(ref day) => match self.used_times.get(&day.as_str()) {
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
                },
                MeetingDay::None => continue,
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
        self.seen.insert(course.subj_course_id.as_str());
        self.sections.push(course);
        for meeting in &course.meetings {
            let end_time = (meeting.end_hr, meeting.end_min);
            let start_time = (meeting.start_hr, meeting.start_min);

            match meeting.meeting_days {
                MeetingDay::Repeated(ref days) => {
                    for day in days {
                        self.used_times
                            .entry(day.as_str())
                            .or_default()
                            .insert((start_time, end_time));
                    }
                }
                MeetingDay::OneTime(ref o) => {
                    self.used_times
                        .entry(o.as_str())
                        .or_default()
                        .insert((start_time, end_time));
                }
                MeetingDay::None => continue,
            }
        }
    }
}

/// Generates all possible schedules. This uses a very naive implementation which
/// will struggle to work on a larger set.
///
/// # Parameters
/// - `wanted_courses`: The desired courses.
/// - `all_courses`: All courses to consider.
/// - `constraints`: Constraints for the schedule generator.
///
/// # Returns
/// A vector containing all schedules.
pub fn generate_schedules<'a>(
    wanted_courses: &[&str],
    all_courses: &'a [CourseSection],
    constraints: ScheduleConstraint,
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
                    let mut s = Schedule::new();
                    for course in all_courses {
                        if !s.can_add_course(course, &constraints) {
                            continue;
                        }

                        s.add_course(course);
                        curr_schedules.push(s);
                        s = Schedule::new();
                    }

                    continue;
                }

                let mut sch_to_add: Vec<Schedule<'a>> = vec![];
                for temp_schedule in &curr_schedules {
                    for course in all_courses {
                        if !temp_schedule.can_add_course(course, &constraints) {
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

/// Constraints for your schedule. Note that this will *not* affect finals time.
pub struct ScheduleConstraint<'a> {
    /// The earliest time any class is allowed to start.
    earliest_start: Option<Time>,
    /// The latest time any class can end.
    latest_end: Option<Time>,
    /// Time between two classes. Note that there is an implicit 10 minute buffer
    /// between classes (e.g. 1:00-1:50, 2:00-2:50)
    buffer_time: Option<i16>,
    /// Any time ranges that you do not want to have classes, discussions, etc.
    off_times: Vec<(&'a str, Time, Time)>,
}

impl<'a> ScheduleConstraint<'a> {
    /// Creates a new `ScheduleConstraint` structure instance.
    ///
    /// # Returns
    /// This new instance.
    pub fn new() -> Self {
        ScheduleConstraint {
            earliest_start: None,
            latest_end: None,
            buffer_time: None,
            off_times: vec![],
        }
    }

    /// Set the earliest time that any given class can start.
    ///
    /// # Parameters
    /// - `hour`: The hour. Must be between 0 and 23, inclusive.
    /// - `min`: The minute. Must be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// This instance.
    pub fn set_earliest_time(mut self, hour: i16, min: i16) -> ScheduleConstraint<'a> {
        if !self._validate_time(hour, min) {
            return self;
        }

        self.earliest_start = Some((hour, min));
        self
    }

    /// Set the latest time that any given class can end.
    ///
    /// # Parameters
    /// - `hour`: The hour. Must be between 0 and 23, inclusive.
    /// - `min`: The minute. Must be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// This instance.
    pub fn set_latest_time(mut self, hour: i16, min: i16) -> ScheduleConstraint<'a> {
        if !self._validate_time(hour, min) {
            return self;
        }

        self.latest_end = Some((hour, min));
        self
    }

    /// Sets the buffer time.
    ///
    /// # Parameters
    /// - `buffer`: The buffer time.
    ///
    /// # Returns
    /// This instance.
    pub fn set_buffer_time(mut self, buffer: i16) -> ScheduleConstraint<'a> {
        self.buffer_time = Some(buffer.abs());
        self
    }

    /// Adds an off-time, or a time range when you don't want classes.
    ///
    /// # Parameters
    /// - `day`: The day of week.
    /// - `start_hour`: The start hour. Must be between 0 and 23, inclusive.
    /// - `start_min`: The start minute. Must be between 0 and 59, inclusive.
    /// - `end_hour`: The end hour. Must be between 0 and 23, inclusive.
    /// - `end_min`: The end minute. Must be between 0 and 59, inclusive.
    ///
    /// # Returns
    /// This instance.
    pub fn add_off_times(
        mut self,
        day: &'a str,
        start_hour: i16,
        start_min: i16,
        end_hour: i16,
        end_min: i16,
    ) -> ScheduleConstraint<'a> {
        if !self._validate_time(start_hour, start_min)
            || !self._validate_time(end_hour, end_min)
            || !DAY_OF_WEEK.contains(&day)
        {
            return self;
        }

        self.off_times
            .push((day, (start_hour, start_min), (end_hour, end_min)));
        self
    }

    fn _validate_time(&self, hour: i16, min: i16) -> bool {
        (0..=23).contains(&hour) && (0..=59).contains(&min)
    }
}
