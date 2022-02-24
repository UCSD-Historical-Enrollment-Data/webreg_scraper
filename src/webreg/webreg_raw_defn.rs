use serde::{Deserialize, Serialize};

/// One possible result you can get by searching for a particular course.
#[derive(Debug, Serialize, Deserialize)]
pub struct WebRegSearchResultItem {
    /// The maximum number of units you can get.
    #[serde(rename = "UNIT_TO")]
    max_units: f32,

    /// The subject code. For example, `CSE` or `MATH` are both possible option.
    #[serde(rename = "SUBJ_CODE")]
    pub subj_code: String,

    /// The course title. For example, `Abstract Algebra II`.
    #[serde(rename = "CRSE_TITLE")]
    pub course_title: String,

    /// The minimum number of units you can get.
    #[serde(rename = "UNIT_FROM")]
    min_units: f32,

    /// The course code. For example, `100B`.
    #[serde(rename = "CRSE_CODE")]
    pub course_code: String,
}

impl ToString for WebRegSearchResultItem {
    fn to_string(&self) -> String {
        format!(
            "[{} {}] {} ({})",
            self.subj_code.trim(),
            self.course_code.trim(),
            self.course_title.trim(),
            self.max_units
        )
    }
}

/// A meeting. Note that this doesn't represent a class by itself, but rather a "piece" of that
/// class. For example, one `WebRegMeeting` can represent a discussion while another can
/// represent a lecture.
#[derive(Debug, Serialize, Deserialize)]
pub struct WebRegMeeting {
    /// The hour part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `11`.
    #[serde(rename = "END_HH_TIME")]
    pub end_time_hr: i16,

    /// The minutes part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `50`.
    #[serde(rename = "END_MM_TIME")]
    pub end_time_min: i16,

    /// The section capacity. For example, if this section has a limit of 196, then this would be
    /// `196`.
    #[serde(rename = "SCTN_CPCTY_QTY")]
    pub section_capacity: i64,

    /// The number of students enrolled in this section.
    #[serde(rename = "SCTN_ENRLT_QTY")]
    pub enrolled_count: i64,

    /// The section number. Each section has a unique number identifier.
    #[serde(rename = "SECTION_NUMBER")]
    pub section_number: String,

    /// The number of students currently on the waitlist.
    #[serde(rename = "COUNT_ON_WAITLIST")]
    pub count_on_waitlist: i64,

    /// The room code. For example, if the meeting is in CENTR 119, then this would be `119`.
    #[serde(rename = "ROOM_CODE")]
    pub room_code: String,

    /// The minute part of the meeting start time. For example, if this meeting starts at 11:00 AM,
    /// then this would be `0`.
    #[serde(rename = "BEGIN_MM_TIME")]
    pub start_time_min: i16,

    /// The hours part of the start time. For example, if this meeting starts at 11:00 AM, then
    /// this would be `11`.
    #[serde(rename = "BEGIN_HH_TIME")]
    pub start_time_hr: i16,

    /// The days that this meeting will take place. This string will only consist of the following:
    /// - `1`: Monday
    /// - `2`: Tuesday
    /// - `3`: Wednesday
    /// - `4`: Thursday
    /// - `5`: Friday
    ///
    /// For example, if a class is meeting MWF, this would be `135`.
    #[serde(rename = "DAY_CODE")]
    pub day_code: String,

    /// The instructor(s).
    #[serde(rename = "PERSON_FULL_NAME")]
    pub person_full_name: String,

    /// Special meeting type, if any. If this is a normal meeting, this will be a string with a
    /// two spaces.
    #[serde(rename = "FK_SPM_SPCL_MTG_CD")]
    pub special_meeting: String,

    /// The building code. For example, if the meeting will take place at Center Hall, this would
    /// be `CENTR`.
    #[serde(rename = "BLDG_CODE")]
    pub bldg_code: String,

    /// The meeting type. See https://registrar.ucsd.edu/StudentLink/instr_codes.html. Note that
    /// this will improperly record final exams, midterms, and other special events as lectures.
    /// So, you need to check `special_meeting` also.
    #[serde(rename = "FK_CDI_INSTR_TYPE")]
    pub meeting_type: String,

    /// The section code. For example, this could be `A00` or `B01`.
    #[serde(rename = "SECT_CODE")]
    pub sect_code: String,

    /// The number of available seats.
    #[serde(rename = "AVAIL_SEAT")]
    pub avail_seat: i64,

    /// The date that this meeting starts. Note that this (`start_date`) and `section_start_date`
    /// will have different dates if the meeting that this `WebRegEvent` represents is a one-day
    /// event (e.g. final exam).
    #[serde(rename = "START_DATE")]
    pub start_date: String,

    /// The date that this section officially starts.
    #[serde(rename = "SECTION_START_DATE")]
    pub section_start_date: String,

    /// How this particular entry is displayed. From my understanding, it looks like:
    /// - `AC`: A section that can be enrolled or planned.
    /// - `NC`: A section that cannot be enrolled or planned (see CSE 8A Discussions).
    /// - `CA`: Canceled.
    #[serde(rename = "FK_SST_SCTN_STATCD")]
    pub display_type: String,

    /// Looks like this flag determines if a section needs to be waitlisted.
    /// - `Y` if the section needs to be waitlisted.
    /// - `N` if the section does not need to be waitlisted.
    #[serde(rename = "STP_ENRLT_FLAG")]
    pub needs_waitlist: String,
}

/// A meeting that you have enrolled in.. Note that this doesn't represent a class by itself, but
/// rather a "piece" of that class. For example, one `ScheduledMeeting` can represent a discussion
/// while another can represent a lecture. Additionally, each `ScheduledMeeting` can only represent
/// one meeting per week (so, for example, a MWF lecture would have 3 entries).
#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduledMeeting {
    // TODO can we guarantee that this will always be an int?
    /// Number of units that this class is being taken for (e.g. 4.00)
    #[serde(rename = "SECTION_HEAD")]
    pub section_number: i64,

    /// Number of units that this class is being taken for (e.g. 4.00)
    #[serde(rename = "SECT_CREDIT_HRS")]
    pub sect_credit_hrs: f32,

    /// The minute part of the meeting start time. For example, if this meeting starts at 11:00 AM,
    /// then this would be `0`.
    #[serde(rename = "BEGIN_MM_TIME")]
    pub start_time_min: i16,

    /// The hours part of the start time. For example, if this meeting starts at 11:00 AM, then
    /// this would be `11`.
    #[serde(rename = "BEGIN_HH_TIME")]
    pub start_time_hr: i16,

    /// The hour part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `11`.
    #[serde(rename = "END_HH_TIME")]
    pub end_time_hr: i16,

    /// The minutes part of the end time. For example, if this meeting ends at 11:50 AM, then
    /// this would be `50`.
    #[serde(rename = "END_MM_TIME")]
    pub end_time_min: i16,

    /// The subject code. For example, `CSE` or `MATH` are both possible option.
    #[serde(rename = "SUBJ_CODE")]
    pub subj_code: String,

    /// The room code. For example, if the meeting is in CENTR 119, then this would be `119`.
    #[serde(rename = "ROOM_CODE")]
    pub room_code: String,

    /// The course title. For example, `Abstract Algebra II`.
    #[serde(rename = "CRSE_TITLE")]
    pub course_title: String,

    /// The grading option. Some common options are `P/NP` or `L`, the former being pass/no pass
    /// and the latter being letter.
    #[serde(rename = "GRADE_OPTION")]
    pub grade_option: String,

    /// The day that this meeting starts. For lectures, this will usually be the first day of the
    /// quarter; for midterms and finals, these will be given different dates.
    #[serde(rename = "START_DATE")]
    pub start_date: String,

    /// The course code. For example, `100B`.
    #[serde(rename = "CRSE_CODE")]
    pub course_code: String,

    /// The day code. Unlike in `WebRegMeeting`, this stores at most 1 number.
    #[serde(rename = "DAY_CODE")]
    pub day_code: String,

    /// The professor teaching this course.
    #[serde(rename = "PERSON_FULL_NAME")]
    pub person_full_name: String,

    /// Special meeting type, if any. If this is a normal meeting, this will be a string with a
    /// two spaces. Note that
    #[serde(rename = "FK_SPM_SPCL_MTG_CD")]
    pub special_meeting: String,

    /// The meeting type. See https://registrar.ucsd.edu/StudentLink/instr_codes.html. Note that
    /// this will properly show the event type.
    #[serde(rename = "FK_CDI_INSTR_TYPE")]
    pub meeting_type: String,

    /// The building code. For example, if the meeting will take place at Center Hall, this would
    /// be `CENTR`.
    #[serde(rename = "BLDG_CODE")]
    pub bldg_code: String,

    /// The current enrollment status. This can be one of:
    /// - `EN`: Enrolled
    /// - `WT`: Waitlisted
    /// - `PL`: Planned
    #[serde(rename = "ENROLL_STATUS")]
    pub enroll_status: String,

    /// The section code. For example, this could be `A00` or `B01`.
    #[serde(rename = "SECT_CODE")]
    pub sect_code: String,

    /// The maximum number of students that can enroll in this section. Note that this is an
    /// `Option` type; this is because this value won't exist if you can't directly enroll in the
    /// section (e.g. you can't directly enroll in a lecture but you can directly enroll in a
    /// lecture + discussion).
    #[serde(rename = "SCTN_CPCTY_QTY")]
    pub section_capacity: Option<i64>,

    /// The number of students enrolled in this section. See `section_capacity` for information.
    #[serde(rename = "SCTN_ENRLT_QTY")]
    pub enrolled_count: Option<i64>,

    /// The number of students currently on the waitlist.
    #[serde(rename = "COUNT_ON_WAITLIST")]
    pub count_on_waitlist: Option<i64>,

    /// Your waitlist position. This will either be an empty string if there is no waitlist,
    /// or your waitlist position if you are on the waitlist.
    #[serde(rename = "WT_POS")]
    pub waitlist_pos: String,
}
