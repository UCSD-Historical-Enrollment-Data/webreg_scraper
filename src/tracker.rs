use crate::util::{get_epoch_time, get_pretty_time};
use crate::{tracker, TermSetting, WebRegHandler};
use crate::{NUM_STOPPED, STOP_FLAG};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::wrapper::{SearchType, WebRegWrapper};

const CLEANED_CSV_HEADER: &str = "time,enrolled,available,waitlisted,total";

#[cfg(debug_assertions)]
const TIMEOUT: [u64; 10] = [5, 8, 16, 32, 64, 128, 256, 512, 1024, 2048];

// The idea is that it should take no more than 15 minutes for
// WebReg to be available.
#[cfg(not(debug_assertions))]
const TIMEOUT: [u64; 10] = [
    // Theoretically, WebReg should be down for no longer than 20 minutes...
    8 * 60,
    6 * 60,
    4 * 60,
    2 * 60,
    // But if WebReg is down for longer, then wait longer...
    10 * 60,
    15 * 60,
    20 * 60,
    30 * 60,
    45 * 60,
    60 * 60,
];

/// Runs the WebReg tracker. This will optionally attempt to reconnect to
/// WebReg when signed out.
///
/// # Parameters
/// - `s`: The wrapper handler.
pub async fn run_tracker(s: &WebRegHandler<'_>) {
    if s.term_setting.apply_term {
        _ = s
            .scraper_wrapper
            .lock()
            .await
            .use_term(s.term_setting.term)
            .await;
        _ = s
            .general_wrapper
            .lock()
            .await
            .use_term(s.term_setting.term)
            .await;
    }

    // In case the given cookies were invalid, if this variable is false, we skip the
    // initial delay and immediately try to fetch the cookies.
    let mut first_passed = false;
    loop {
        tracker::track_webreg_enrollment(&s.scraper_wrapper, s.term_setting).await;

        if STOP_FLAG.load(Ordering::SeqCst) {
            break;
        }

        // If we're here, this means something went wrong.
        let port = match s.term_setting.port {
            Some(p) => p,
            None => break,
        };

        // Basically, keep on trying until we get back into WebReg.
        let mut success = false;
        for time in TIMEOUT {
            if first_passed {
                println!(
                    "[{}] [{}] Taking a {} second break.",
                    s.term_setting.term,
                    get_pretty_time(),
                    time
                );
                tokio::time::sleep(Duration::from_secs(time)).await;
            }

            first_passed = true;

            // Get new cookies.
            let new_cookie_str = {
                match reqwest::get(format!("http://localhost:{}/cookie", port)).await {
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

                s.scraper_wrapper.lock().await.set_cookies(c.clone());
                s.general_wrapper.lock().await.set_cookies(c);

                if s.term_setting.apply_term {
                    _ = s
                        .scraper_wrapper
                        .lock()
                        .await
                        .use_term(s.term_setting.term)
                        .await;
                    _ = s
                        .general_wrapper
                        .lock()
                        .await
                        .use_term(s.term_setting.term)
                        .await;
                }
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

    // This should only run if we're 100% done with this
    // wrapper. For example, either the wrapper could not
    // log back in or we forced it to stop.
    NUM_STOPPED.fetch_add(1, Ordering::SeqCst);
    println!(
        "[{}] [{}] Quitting.",
        s.term_setting.term,
        get_pretty_time()
    );
}

/// Tracks WebReg for enrollment information. This will continuously check specific courses for
/// their enrollment information (number of students waitlisted/enrolled, total seats) along with
/// basic course information and store this in a CSV file for later processing.
///
/// # Parameters
/// - `wrapper`: The wrapper.
/// - `setting`: The settings for this term.
/// - `end_location`: The end location for the cleaned CSV files. Just the base location will
///   suffice.
pub async fn track_webreg_enrollment(wrapper: &Mutex<WebRegWrapper>, setting: &TermSetting<'_>) {
    // If the wrapper doesn't have a valid cookie, then return.
    if !wrapper.lock().await.is_valid().await {
        eprintln!(
            "[{}] [{}] Initial instance is not valid. Returning.",
            setting.term,
            get_pretty_time()
        );

        return;
    }

    let file_name = format!(
        "enrollment_{}_{}.csv",
        chrono::offset::Local::now().format("%FT%H_%M_%S"),
        setting.alias.unwrap_or(setting.term)
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
        let mut results = vec![];

        for search_query in &setting.search_query {
            let mut temp = w
                .search_courses(SearchType::Advanced(search_query))
                .await
                .unwrap_or_default();

            results.append(&mut temp);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Drop the Mutex to unlock it
        drop(w);

        if results.is_empty() {
            eprintln!(
                "[{}] [{}] No courses found. Exiting.",
                setting.term,
                get_pretty_time()
            );
            break;
        }

        println!(
            "[{}] [{}] Found {} results successfully.",
            setting.term,
            get_pretty_time(),
            results.len()
        );

        for r in results {
            if STOP_FLAG.load(Ordering::SeqCst) {
                break 'main;
            }

            if fail_count != 0 && fail_count > 12 {
                eprintln!(
                    "[{}] [{}] Too many failures when trying to request data from WebReg.",
                    setting.term,
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
                        setting.term,
                        get_pretty_time(),
                        e,
                        fail_count
                    );
                }
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    println!(
                        "[{}] [{}] Processing {} section(s) for {}: [{}]",
                        setting.term,
                        get_pretty_time(),
                        r.len(),
                        r[0].subj_course_id,
                        r.iter()
                            .map(|data| data.section_id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );

                    let time = get_epoch_time();
                    // Write to raw CSV dataset
                    r.iter().for_each(|c| {
                        writeln!(
                            writer,
                            "{},{},{},{},{},{},{},{},{}",
                            time,
                            c.subj_course_id,
                            c.section_code,
                            c.section_id,
                            // Every instructor name (except staff) has a comma
                            c.all_instructors.join(" & ").replace(',', ";"),
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
                        setting.term,
                        get_pretty_time(),
                        r.subj_code.trim(),
                        r.course_code.trim(),
                        fail_count
                    );
                }
            }

            // Sleep between requests so we don't get ourselves banned by webreg
            tokio::time::sleep(Duration::from_secs_f64(setting.cooldown)).await;
        }
    }

    if !writer.buffer().is_empty() {
        println!(
            "[{}] [{}] Buffer not empty! Buffer has length {}.",
            setting.term,
            get_pretty_time(),
            writer.buffer().len()
        );
    }

    writer.flush().unwrap();
    // Debugging possible issues with the buffer
    println!(
        "[{}] [{}] Buffer flushed. Final buffer length: {}.",
        setting.term,
        get_pretty_time(),
        writer.buffer().len()
    );
}

struct CourseFile {
    /// The file containing data combined from *all* sections.
    overall_file: File,

    /// The file**s** containing data for each section family.
    section_files: HashMap<String, File>,
}

// in the form: (enrolled, available, waitlisted, total)
#[derive(Clone, Copy, Default)]
struct CourseStat(i64, i64, i64, i64);

impl Add<CourseStat> for CourseStat {
    type Output = CourseStat;

    fn add(self, rhs: CourseStat) -> Self::Output {
        CourseStat(
            self.0 + rhs.0,
            self.1 + rhs.1,
            self.2 + rhs.2,
            self.3 + rhs.3,
        )
    }
}

impl AddAssign<CourseStat> for CourseStat {
    fn add_assign(&mut self, rhs: CourseStat) {
        self.0 += rhs.0;
        self.1 += rhs.1;
        self.2 += rhs.2;
        self.3 += rhs.3;
    }
}

impl Sum for CourseStat {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        // For our use case here, it is assumed that we have at least one element.
        iter.reduce(|prev, next| prev + next).unwrap_or_default()
    }
}
