use crate::webreg::webreg_clean_defn::MeetingDay;
use crate::webreg::webreg_raw_defn::WebRegMeeting;

/// Checks if this is a valid WebReg meeting. This, in particular, checks to make sure the times
/// are not all 0. If they are, this implies that the section was canceled.
///
/// # Parameters
/// - `webreg_meeting`: The WebReg meeting to check.
///
/// # Returns
/// `true` if this is a valid meeting and `false` otherwise.
#[inline]
pub fn is_valid_meeting(webreg_meeting: &WebRegMeeting) -> bool {
    webreg_meeting.start_time_min != 0
        || webreg_meeting.start_time_hr != 0
        || webreg_meeting.end_time_min != 0
        || webreg_meeting.end_time_hr != 0
        || webreg_meeting.section_capacity != 0
}

/// Gets the meeting type (e.g. Lecture, Final Exam, Discussion, etc.) and the meeting time from
/// an arbitrary `WebRegMeeting`.
///
/// # Parameters
/// - `w_meeting`: The WebReg meeting to check.
///
/// # Returns
/// A tuple where:
/// - the first element is the meeting type
/// - the second element is/are the day(s) that this meeting occurs
#[inline]
pub fn parse_meeting_type_date(w_meeting: &WebRegMeeting) -> (&str, MeetingDay) {
    let special_meeting = w_meeting.special_meeting.trim();
    if !special_meeting.is_empty() && special_meeting != "TBA" {
        assert!(!w_meeting.section_start_date.is_empty());
        return (
            special_meeting,
            MeetingDay::OneTime(w_meeting.start_date.to_string()),
        );
    }

    // assert_eq!(w_meeting.section_start_date, w_meeting.start_date);

    let regular_meeting = w_meeting.meeting_type.trim();
    let day_code = w_meeting.day_code.trim();
    assert!(day_code.chars().into_iter().all(|x| x.is_numeric()));

    if day_code.is_empty() {
        (regular_meeting, MeetingDay::None)
    } else {
        (
            regular_meeting,
            MeetingDay::Repeated(parse_day_code(day_code)),
        )
    }
}

/// Parses the days of the week from a day code string.
///
/// # Parameters
/// - `dow_str`: The day code string. This should only contain integers between 0 and 6, both
/// inclusive.
///
/// # Returns
/// A string with the days of the week.
pub fn parse_day_code(day_code_str: &str) -> String {
    let mut s: String = String::new();
    day_code_str.chars().for_each(|c| {
        if !c.is_numeric() {
            return;
        }

        match c {
            '0' => s.push_str("Su"),
            '1' => s.push_str("M"),
            '2' => s.push_str("Tu"),
            '3' => s.push_str("W"),
            '4' => s.push_str("Th"),
            '5' => s.push_str("F"),
            '6' => s.push_str("Sa"),
            _ => {}
        };
    });

    s
}
