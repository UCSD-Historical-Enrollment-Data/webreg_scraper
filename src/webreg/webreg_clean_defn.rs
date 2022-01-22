use serde::Serialize;

/// A section, which consists of a lecture, usually a discussion, and usually a final.
#[derive(Debug, Clone, Serialize)]
pub struct CourseSection {
    pub section_id: String,
    pub section_code: String,
    pub instructor: String,
    pub available_seats: i64,
    pub total_seats: i64,
    pub waitlist_ct: i64,
    pub meetings: Vec<Meeting>,
}

impl ToString for CourseSection {
    fn to_string(&self) -> String {
        let mut s = format!(
            "[{} / {}] {}: {}/{} (WL: {})\n",
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
    pub meeting_type: String,
    #[serde(rename = "meeting_days")]
    pub meeting_days: MeetingDay,
    pub start_min: i16,
    pub start_hr: i16,
    pub end_min: i16,
    pub end_hr: i16,
    pub building: String,
    pub room: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MeetingDay {
    Repeated(String),
    OneTime(String),
    None,
}

impl ToString for Meeting {
    fn to_string(&self) -> String {
        let meeting_days_display = match &self.meeting_days {
            MeetingDay::Repeated(r) => r.as_str(),
            MeetingDay::OneTime(r) => r.as_str(),
            MeetingDay::None => "N/A",
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
