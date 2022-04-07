use crate::util::{get_epoch_time, get_pretty_time};
use crate::{tracker, BASE_COOLDOWN, TERMS};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::webreg_wrapper::{CourseLevelFilter, SearchRequestBuilder, SearchType, WebRegWrapper};

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
/// - `term`: The term associated with this tracker.
pub async fn run_tracker(w: Arc<Mutex<WebRegWrapper<'_>>>, cookie_url: Option<&str>, term: &str) {
    // In case the given cookies were invalid, if this variable is false, we skip the
    // initial delay and immediately try to fetch the cookies.
    let mut first_passed = false;
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
            term,
        )
        .await;

        // If we're here, this means something went wrong.
        if cookie_url.is_none() {
            break;
        }

        // Basically, keep on trying until we get back into WebReg.
        let mut success = false;
        for time in TIMEOUT {
            if first_passed {
                println!("[{}] [{}] Taking a {} second break.", term, get_pretty_time(), time);
                tokio::time::sleep(Duration::from_secs(time)).await;
            }

            first_passed = true;

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

                w.lock().await.set_cookies(c);
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

    println!("[{}] [{}] Quitting.", term, get_pretty_time());
}

/// Tracks WebReg for enrollment information. This will continuously check specific courses for
/// their enrollment information (number of students waitlisted/enrolled, total seats) along with
/// basic course information and store this in a CSV file for later processing.
///
/// # Parameters
/// - `wrapper`: The wrapper.
/// - `search_res`: The courses to search for.
/// - `term`: The term associated with this tracker.
pub async fn track_webreg_enrollment(
    wrapper: &Arc<Mutex<WebRegWrapper<'_>>>,
    search_res: &SearchRequestBuilder<'_>,
    term: &str,
) {
    // If the wrapper doesn't have a valid cookie, then return.
    if !wrapper.lock().await.is_valid().await {
        eprintln!(
            "[{}] [{}] Initial instance is not valid. Returning.",
            term,
            get_pretty_time()
        );

        return;
    }

    let file_name = format!(
        "enrollment_{}_{}.csv",
        chrono::offset::Local::now().format("%FT%H_%M_%S"),
        term
    );
    let is_new = !Path::new(&file_name).exists();

    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_name)
        .unwrap_or_else(|_| panic!("could not open or create '{}'", file_name));

    let mut writer = BufWriter::new(f);
    if is_new {
        writeln!(
            writer,
            "time,subj_course_id,sec_code,sec_id,prof,available,waitlist,total,enrolled_ct"
        )
        .unwrap();
    }

    let mut fail_count = 0;
    'main: loop {
        writer.flush().unwrap();
        let w = wrapper.lock().await;
        let results = w
            .search_courses(SearchType::Advanced(search_res))
            .await
            .unwrap_or_default();
        // Drop the Mutex to unlock it
        drop(w);

        if results.is_empty() {
            eprintln!(
                "[{}] [{}] No courses found. Exiting.",
                term,
                get_pretty_time()
            );
            break;
        }

        println!(
            "[{}] [{}] Found {} results successfully.",
            term,
            get_pretty_time(),
            results.len()
        );

        for r in results {
            if fail_count != 0 && fail_count > 12 {
                eprintln!(
                    "[{}] [{}] Too many failures when trying to request data from WebReg.",
                    term,
                    get_pretty_time()
                );
                break 'main;
            }

            let w = wrapper.lock().await;
            let res = w
                .get_enrollment_count(r.subj_code.trim(), r.course_code.trim())
                .await;
            drop(w);

            match res {
                Err(e) => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] [{}] An error occurred ({}). Skipping. (FAIL_COUNT: {})",
                        term,
                        get_pretty_time(),
                        e,
                        fail_count
                    );
                }
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    println!(
                        "[{}] [{}] Processing {} section(s) for {}.",
                        term,
                        get_pretty_time(),
                        r.len(),
                        r[0].subj_course_id
                    );

                    let time = get_epoch_time();
                    r.into_iter().for_each(|c| {
                        writeln!(
                            writer,
                            "{},{},{},{},{},{},{},{},{}",
                            time,
                            c.subj_course_id,
                            c.section_code,
                            c.section_id,
                            // Every instructor name (except staff) has a comma
                            c.instructor.join(" & ").replace(",", ";"),
                            c.available_seats,
                            c.waitlist_ct,
                            c.total_seats,
                            c.enrolled_ct,
                        )
                        .unwrap()
                    });
                }
                _ => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] [{}] Course {} {} not found. Were you logged out? (FAIL_COUNT: {}).",
                        term,
                        get_pretty_time(),
                        r.subj_code,
                        r.course_code,
                        fail_count
                    );
                }
            }

            // Just to be nice to webreg
            tokio::time::sleep(Duration::from_secs((TERMS.len() * BASE_COOLDOWN) as u64)).await;
        }
    }

    writer.flush().unwrap();
}
