use serde::Serialize;

/// A section, which consists of a lecture, usually a discussion, and usually a final.
#[derive(Debug, Clone, Serialize)]
pub struct CourseSection {
    /// The subject, course ID. For example, `CSE 100`.
    pub subj_course_id: String,
    /// The section ID. For example, `079912`.
    pub section_id: String,
    /// The section code. For example, `B01`.
    pub section_code: String,
    /// The instructor.
    pub instructor: String,
    /// The number of available seats.
    pub available_seats: i64,
    /// The total number of seats.
    pub total_seats: i64,
    /// The waitlist count.
    pub waitlist_ct: i64,
    /// All meetings.
    pub meetings: Vec<Meeting>,
}

impl ToString for CourseSection {
    fn to_string(&self) -> String {
        let mut s = format!(
            "[{}] [{} / {}] {}: {}/{} (WL: {})\n",
            self.subj_course_id,
            self.section_code,
            self.section_id,
            self.instructor,
            self.available_seats,
            self.total_seats,
            self.waitlist_ct
        );

        for meeting in &self.meetings {
            s.push_str(&*meeting.to_string());
            s.push('\n');
        }

        s
    }
}

/// A meeting.
#[derive(Debug, Clone, Serialize)]
pub struct Meeting {
    /// The meeting type. For example, this can be `LE`, `FI`, `DI`, etc.
    pub meeting_type: String,
    /// The meeting day(s). This is an enum that represents either a reoccurring meeting
    /// or one-time meeting.
    #[serde(rename = "meeting_days")]
    pub meeting_days: MeetingDay,
    /// The start hour. For example, if the meeting starts at 14:15, this would be `14`.
    pub start_hr: i16,
    /// The start minute. For example, if the meeting starts at 14:15, this would be `15`.
    pub start_min: i16,
    /// The end hour. For example, if the meeting ends at 15:05, this would be `15`.
    pub end_hr: i16,
    /// The end minute. For example, if the meeting ends at 15:05, this would be `5`.
    pub end_min: i16,
    /// The building where this meeting will occur. For example, if the meeting is held in
    /// `CENTR 115`, then this would be `CENTR`.
    pub building: String,
    /// The room number where this meeting will occur. For example, if the meeting is held in
    /// `CENTR 115`, then this would be `115`.
    pub room: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MeetingDay {
    Repeated(Vec<String>),
    OneTime(String),
    None,
}

impl Meeting {
    /// Returns a flat string representation of this `Meeting`
    ///
    /// # Returns
    /// A flat string representation of this `Meeting`. Useful for CSV files.
    pub fn to_flat_str(&self) -> String {
        let mut s = String::new();
        s.push_str(&match &self.meeting_days {
            MeetingDay::Repeated(r) => r.join(""),
            MeetingDay::OneTime(r) => r.to_string(),
            MeetingDay::None => "N/A".to_string(),
        });

        s.push(' ');
        s.push_str(self.meeting_type.as_str());
        s.push(' ');
        s.push_str(&format!(
            "{}:{:02} - {}:{:02}",
            self.start_hr, self.start_min, self.end_hr, self.end_min
        ));

        s
    }
}

impl ToString for Meeting {
    fn to_string(&self) -> String {
        let meeting_days_display = match &self.meeting_days {
            MeetingDay::Repeated(r) => r.join(""),
            MeetingDay::OneTime(r) => r.to_string(),
            MeetingDay::None => "N/A".to_string(),
        };

        let time_range = format!(
            "{}:{:02} - {}:{:02}",
            self.start_hr, self.start_min, self.end_hr, self.end_min
        );
        format!(
            "\t[{}] {} at {} in {} {}",
            self.meeting_type, meeting_days_display, time_range, self.building, self.room
        )
    }
}

/// A section that is currently in your schedule. Note that this can either be a course that you
/// are enrolled in, waitlisted for, or planned.
#[derive(Debug, Clone, Serialize)]
pub struct ScheduledSection {
    /// The section number, for example `79903`.
    pub section_number: i64,
    /// The subject code. For example, if this represents `CSE 100`, then this would be `CSE`.
    pub subject_code: String,
    /// The subject code. For example, if this represents `CSE 100`, then this would be `100`.
    pub course_code: String,
    /// The course title, for example `Advanced Data Structure`.
    pub course_title: String,
    /// The section code, for example `A01`.
    pub section_code: String,
    /// The section capacity (maximum number of people that can enroll in this section).
    pub section_capacity: i64,
    /// The number of people enrolled in this section.
    pub enrolled_count: i64,
    /// The grading option. This can be one of `L`, `P`, or `S`.
    pub grade_option: String,
    /// The instructor for this course.
    pub instructor: String,
    /// The number of units that you are taking this course for.
    pub units: f32,
    /// Your enrollment status.
    #[serde(rename = "enrolled_status")]
    pub enrolled_status: EnrollmentStatus,
    /// The number of people on the waitlist.
    pub waitlist_ct: i64,
    /// All relevant meetings for this section.
    pub meetings: Vec<Meeting>,
}

impl ToString for ScheduledSection {
    fn to_string(&self) -> String {
        let status = match self.enrolled_status {
            EnrollmentStatus::Enrolled => "Enrolled",
            EnrollmentStatus::Waitlist(_) => "Waitlisted",
            EnrollmentStatus::Planned => "Planned",
        };

        let mut s = format!(
            "[{} / {}] {} ({} {}) with {} - {} ({} Units, {} Grading)\n",
            self.section_code,
            self.section_number,
            self.course_title,
            self.subject_code,
            self.course_code,
            self.instructor,
            status,
            self.units,
            self.grade_option
        );

        for meeting in &self.meetings {
            s.push_str(&*meeting.to_string());
            s.push('\n');
        }

        s
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EnrollmentStatus {
    Enrolled,
    Waitlist(i64),
    Planned,
}
