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
) -> () {
    let is_new = !Path::new(ENROLLMENT_NAME).exists();

    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(ENROLLMENT_NAME)
        .expect("could not open or create 'enrollment.csv'");

    let mut writer = BufWriter::new(f);
    if is_new {
        writer
            .write_fmt(format_args!(
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
            ))
            .unwrap();
    }

    let mut i = 0;
    loop {
        writer.flush().unwrap();
        let results = wrapper.search_courses(search_res).await.unwrap_or(vec![]);

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
            if i % 10 == 0 {
                println!("[{}] Taking a 2 second break.", get_pretty_time());
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            let res = wrapper
                .get_course_info(r.subj_code.trim(), r.course_code.trim())
                .await;
            match res {
                None => panic!(
                    "[{}] WebReg authentication error occurred.",
                    get_pretty_time()
                ),
                Some(r) if r.len() > 0 => {
                    println!(
                        "[{}] Processing {} section(s) for {}.",
                        get_pretty_time(),
                        r.len(),
                        r[0].course_dept_id
                    );

                    r.into_iter().for_each(|c| {
                        write!(
                            writer,
                            "{},{},{},{},{},{},{},{},{}\n",
                            get_epoch_time(),
                            c.course_dept_id,
                            c.section_code,
                            c.section_id,
                            c.instructor,
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

                                    s.push_str(" ");
                                    s.push_str(&m.meeting_type);
                                    s.push_str(" ");
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
                _ => eprintln!(
                    "[{}] Course {} {} not found on WebReg.",
                    get_pretty_time(),
                    r.subj_code,
                    r.course_code
                ),
            }

            i += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
