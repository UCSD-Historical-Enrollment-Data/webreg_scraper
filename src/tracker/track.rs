use crate::util::{get_epoch_time, get_pretty_time};
use crate::webreg::webreg_clean_defn::MeetingDay;
use crate::{SearchRequestBuilder, WebRegWrapper};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;
use tokio;

const ENROLLMENT_NAME: &str = "enrollment.csv";

/// Tracks WebReg for enrollment information. This will continuously check specific courses for
/// their enrollment information (number of students waitlisted/enrolled, total seats) along with
/// basic course information and store this in a CSV file for later processing.
///
/// # Parameters
/// - `wrapper`: The wrapper.
/// - `search_res`: The courses to search for.
pub async fn track_webreg_enrollment(
    wrapper: &WebRegWrapper<'_>,
    search_res: &SearchRequestBuilder<'_>,
) {
    let is_new = !Path::new(ENROLLMENT_NAME).exists();

    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(ENROLLMENT_NAME)
        .expect("could not open or create 'enrollment.csv'");

    let mut writer = BufWriter::new(f);
    if is_new {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{}\n",
            "time",
            "subj_course_id",
            "sec_code",
            "sec_id",
            "prof",
            "available",
            "waitlist",
            "total",
            "meetings"
        )
        .unwrap();
    }

    let mut fail_count = 0;
    loop {
        if fail_count != 0 && fail_count > 20 {
            eprintln!(
                "[{}] Too many failures when trying to request data from WebReg. Quitting.",
                get_pretty_time()
            );
            break;
        }

        writer.flush().unwrap();
        let results = wrapper.search_courses(search_res).await.unwrap_or_default();

        if results.is_empty() {
            eprintln!("[{}] No courses found. Exiting.", get_pretty_time());
            break;
        }

        println!(
            "[{}] Found {} results successfully.",
            get_pretty_time(),
            results.len()
        );

        for r in results {
            let res = wrapper
                .get_course_info(r.subj_code.trim(), r.course_code.trim())
                .await;
            match res {
                None => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] An unknown error occurred. Were you logged out? Skipping. (FAIL_COUNT: {})",
                        get_pretty_time(),
                        fail_count
                    );
                }
                Some(r) if !r.is_empty() => {
                    fail_count = 0;
                    println!(
                        "[{}] Processing {} section(s) for {}.",
                        get_pretty_time(),
                        r.len(),
                        r[0].course_dept_id
                    );

                    r.into_iter().for_each(|c| {
                        writeln!(
                            writer,
                            "{},{},{},{},{},{},{},{},{}",
                            get_epoch_time(),
                            c.course_dept_id,
                            c.section_code,
                            c.section_id,
                            // Every instructor name (except staff) has a comma
                            c.instructor.replace(",", ";"),
                            c.available_seats,
                            c.waitlist_ct,
                            c.total_seats,
                            c.meetings
                                .into_iter()
                                .map(|m| {
                                    let mut s = String::new();
                                    s.push_str(&match m.meeting_days {
                                        MeetingDay::Repeated(r) => r.join(""),
                                        MeetingDay::OneTime(r) => r,
                                        MeetingDay::None => "N/A".to_string(),
                                    });

                                    s.push(' ');
                                    s.push_str(&m.meeting_type);
                                    s.push(' ');
                                    s.push_str(&format!(
                                        "{}:{:02} - {}:{:02}",
                                        m.start_hr, m.start_min, m.end_hr, m.end_min
                                    ));

                                    s
                                })
                                .collect::<Vec<_>>()
                                .join("|")
                        )
                        .unwrap()
                    });
                }
                _ => {
                    fail_count = 0;
                    eprintln!(
                        "[{}] Course {} {} not found on WebReg.",
                        get_pretty_time(),
                        r.subj_code,
                        r.course_code
                    );
                }
            }

            // Just to be nice to webreg
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}
