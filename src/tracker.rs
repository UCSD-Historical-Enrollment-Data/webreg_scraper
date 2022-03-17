use crate::util::{get_epoch_time, get_pretty_time};
use crate::webreg::webreg_wrapper::{CourseLevelFilter, SearchType};
use crate::{SearchRequestBuilder, tracker, WebRegWrapper};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;
use serde_json::Value;

#[cfg(debug_assertions)]
const TIMEOUT: [u64; 3] = [5, 10, 15];

#[cfg(not(debug_assertions))]
// The idea is that it should take no more than 15 minutes for
// WebReg to be available.
const TIMEOUT: [u64; 3] = [8 * 60, 6 * 60, 4 * 60];


/// Runs the WebReg tracker. This will optionally attempt to reconnect to
/// WebReg when signed out.
///
/// # Parameters
/// - `w`: The wrapper.
/// - `cookie_url`: The URL to the API where new cookies can be requested. If none
/// is specified, then this will automatically terminate upon any issue with the
/// tracker.
pub async fn run_tracker(w: &mut WebRegWrapper<'_>, cookie_url: Option<&str>) {
    loop {
        tracker::track_webreg_enrollment(
            &w,
            &SearchRequestBuilder::new()
                .add_subject("CSE")
                .add_subject("COGS")
                .add_subject("MATH")
                .add_subject("ECE")
                .filter_courses_by(CourseLevelFilter::LowerDivision)
                .filter_courses_by(CourseLevelFilter::UpperDivision),
        )
            .await;

        // If we're here, this means something went wrong.
        if cookie_url.is_none() {
            break;
        }

        // Basically, keep on trying until we get back into WebReg.
        let mut success = false;
        for time in TIMEOUT {
            println!("[{}] Taking a {} second break.", get_pretty_time(), time);
            tokio::time::sleep(Duration::from_secs(time)).await;

            // Get new cookies.
            let new_cookie_str = {
                match reqwest::get(cookie_url.unwrap()).await {
                    Ok(t) => {
                        let txt = t.text().await.unwrap_or_default();
                        let json: Value = serde_json::from_str(&txt).unwrap_or_default();
                        if json["cookie"].is_string() {
                            Some(json["cookie"].as_str().unwrap().to_string())
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            };

            // And then try to make a new wrapper with said cookies.
            if let Some(c) = new_cookie_str {
                // Empty string = failed to get data.
                // Try again.
                if c.is_empty() {
                    continue;
                }

                w.set_cookies(c);
                success = true;
                break;
            }
        }

        // If successful, we can continue pinging WebReg.
        if success {
            continue;
        }

        // Otherwise, gracefully quit.
        break;
    }

    println!("[{}] Quitting.", get_pretty_time());
}


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
    let file_name = format!(
        "enrollment_{}.csv",
        chrono::offset::Local::now().format("%FT%H_%M_%S")
    );
    let is_new = !Path::new(&file_name).exists();

    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_name)
        .expect("could not open or create 'enrollment.csv'");

    let mut writer = BufWriter::new(f);
    if is_new {
        writeln!(
            writer,
            "time,subj_course_id,sec_code,sec_id,prof,available,waitlist,total"
        )
        .unwrap();
    }

    let mut fail_count = 0;
    'main: loop {
        writer.flush().unwrap();
        let results = wrapper
            .search_courses(SearchType::Advanced(search_res))
            .await
            .unwrap_or_default();

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
            if fail_count != 0 && fail_count > 20 {
                eprintln!(
                    "[{}] Too many failures when trying to request data from WebReg. Exiting.",
                    get_pretty_time()
                );
                break 'main;
            }

            let res = wrapper
                .get_enrollment_count(r.subj_code.trim(), r.course_code.trim())
                .await;
            match res {
                Err(e) => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] An error occurred ({}). Skipping. (FAIL_COUNT: {})",
                        get_pretty_time(),
                        e,
                        fail_count
                    );
                }
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    println!(
                        "[{}] Processing {} section(s) for {}.",
                        get_pretty_time(),
                        r.len(),
                        r[0].subj_course_id
                    );

                    let time = get_epoch_time();
                    r.into_iter().for_each(|c| {
                        writeln!(
                            writer,
                            "{},{},{},{},{},{},{},{}",
                            time,
                            c.subj_course_id,
                            c.section_code,
                            c.section_id,
                            // Every instructor name (except staff) has a comma
                            c.instructor.replace(",", ";"),
                            c.available_seats,
                            c.waitlist_ct,
                            c.total_seats,
                        )
                        .unwrap()
                    });
                }
                _ => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] Course {} {} not found on WebReg. Were you logged out? (FAIL_COUNT: {}).",
                        get_pretty_time(),
                        r.subj_code,
                        r.course_code,
                        fail_count
                    );
                }
            }

            // Just to be nice to webreg
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }

    writer.flush().unwrap();
}
