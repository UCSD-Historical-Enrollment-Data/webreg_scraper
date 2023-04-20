use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::time::Instant;
use webweg::wrapper::SearchType;

#[cfg(feature = "scraper")]
use {
    crate::util::get_epoch_time,
    std::collections::HashMap,
    std::fs::{File, OpenOptions},
    std::io::{BufWriter, Write},
    std::iter::Sum,
    std::ops::{Add, AddAssign},
    std::path::Path,
};

use crate::types::TermInfo;
use crate::util::get_pretty_time;

const MAX_RECENT_REQUESTS: usize = 2000;
const CLEANED_CSV_HEADER: &str = "time,enrolled,available,waitlisted,total";

#[cfg(debug_assertions)]
const LOGIN_TIMEOUT: [u64; 10] = [5, 8, 16, 32, 64, 128, 256, 512, 1024, 2048];

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
pub async fn run_tracker(wrapper_info: Arc<TermInfo>, stop_flag: Arc<AtomicBool>, verbose: bool) {
    if wrapper_info.apply_term {
        let _ = wrapper_info
            .scraper_wrapper
            .lock()
            .await
            .use_term(wrapper_info.term.as_str())
            .await;
        let _ = wrapper_info
            .general_wrapper
            .lock()
            .await
            .use_term(wrapper_info.term.as_str())
            .await;
    }

    // In case the given cookies were invalid, if this variable is false, we skip the
    // initial delay and immediately try to fetch the cookies.
    let mut first_passed = false;
    loop {
        wrapper_info.is_running.store(true, Ordering::SeqCst);
        track_webreg_enrollment(&wrapper_info, &stop_flag, verbose).await;
        wrapper_info.is_running.store(false, Ordering::SeqCst);

        if stop_flag.load(Ordering::SeqCst) {
            break;
        }

        // If we're here, this means something went wrong.
        let address = format!(
            "{}:{}",
            wrapper_info.recovery.address, wrapper_info.recovery.port
        );

        // Basically, keep on trying until we get back into WebReg.
        let mut success = false;
        for time in LOGIN_TIMEOUT {
            if first_passed {
                println!(
                    "[{}] [{}] Taking a {} second break.",
                    wrapper_info.term,
                    get_pretty_time(),
                    time
                );
                tokio::time::sleep(Duration::from_secs(time)).await;
            }

            first_passed = true;

            // Get new cookies.
            let new_cookie_str = {
                match webweg::reqwest::get(format!("http://{address}/cookie")).await {
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

                wrapper_info
                    .scraper_wrapper
                    .lock()
                    .await
                    .set_cookies(c.clone());
                wrapper_info.general_wrapper.lock().await.set_cookies(c);

                if wrapper_info.apply_term {
                    let _ = wrapper_info
                        .scraper_wrapper
                        .lock()
                        .await
                        .use_term(&wrapper_info.term)
                        .await;
                    let _ = wrapper_info
                        .general_wrapper
                        .lock()
                        .await
                        .use_term(&wrapper_info.term)
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
    println!("[{}] [{}] Quitting.", wrapper_info.term, get_pretty_time());
}

/// Tracks WebReg for enrollment information. This will continuously check specific courses for
/// their enrollment information (number of students waitlisted/enrolled, total seats) along with
/// basic course information and store this in a CSV file for later processing.
///
/// # Parameters
/// - `info`: The term information.
/// - `stop_flag`: The stop flag. This is essentially a global flag that indicates if the scraper
/// should stop running.
/// - `verbose`: Whether logging should be verbose.
pub async fn track_webreg_enrollment(info: &TermInfo, stop_flag: &Arc<AtomicBool>, verbose: bool) {
    // If the wrapper doesn't have a valid cookie, then return.
    if !info.scraper_wrapper.lock().await.is_valid().await {
        eprintln!(
            "[{}] [{}] Initial instance is not valid. Returning.",
            info.term,
            get_pretty_time()
        );

        return;
    }

    #[cfg(feature = "scraper")]
    let mut writer = {
        let file_name = format!(
            "enrollment_{}_{}.csv",
            chrono::offset::Local::now().format("%FT%H_%M_%S"),
            info.alias.as_ref().unwrap_or(&info.term)
        );
        let is_new = !Path::new(&file_name).exists();

        let f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&file_name)
            .unwrap_or_else(|_| panic!("could not open or create '{file_name}'"));

        let mut w = BufWriter::new(f);
        if is_new {
            writeln!(
                w,
                "time,subj_course_id,sec_code,sec_id,prof,available,waitlist,total,enrolled_ct"
            )
            .unwrap();
        }

        w
    };

    let mut fail_count = 0;
    'main: loop {
        #[cfg(feature = "scraper")]
        writer.flush().unwrap();
        let results = {
            let mut r = vec![];
            let w = info.scraper_wrapper.lock().await;
            for search_query in &info.search_query {
                let mut temp = w
                    .search_courses(SearchType::Advanced(search_query))
                    .await
                    .unwrap_or_default();

                r.append(&mut temp);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            r
        };

        if results.is_empty() {
            eprintln!(
                "[{}] [{}] No courses found. Exiting.",
                info.term,
                get_pretty_time()
            );
            break;
        }

        #[cfg(feature = "scraper")]
        println!(
            "[{}] [{}] Found {} results successfully.",
            info.term,
            get_pretty_time(),
            results.len()
        );
        #[cfg(not(feature = "scraper"))]
        println!(
            "[{}] [{}] Search execution successful ({}).",
            info.term,
            get_pretty_time(),
            results.len()
        );

        for r in results {
            if stop_flag.load(Ordering::SeqCst) {
                break 'main;
            }

            if fail_count != 0 && fail_count > 12 {
                eprintln!(
                    "[{}] [{}] Too many failures when trying to request data from WebReg.",
                    info.term,
                    get_pretty_time()
                );
                break 'main;
            }

            // Start timing.
            let start_time = Instant::now();

            let res = {
                let w = info.scraper_wrapper.lock().await;
                w.get_enrollment_count(r.subj_code.trim(), r.course_code.trim())
                    .await
            };

            match res {
                Err(e) => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] [{}] An error occurred ({}). Skipping. (FAIL_COUNT: {})",
                        info.term,
                        get_pretty_time(),
                        e,
                        fail_count
                    );
                }
                #[cfg(not(feature = "scraper"))]
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    if verbose {
                        println!(
                            "[{}] [{}] Pinged successfully!",
                            info.term,
                            get_pretty_time(),
                        );
                    }
                }
                #[cfg(feature = "scraper")]
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    if verbose {
                        println!(
                            "[{}] [{}] Processing {} section(s) for {}",
                            info.term,
                            get_pretty_time(),
                            r.len(),
                            r[0].subj_course_id
                        );
                    }

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
                        info.term,
                        get_pretty_time(),
                        r.subj_code.trim(),
                        r.course_code.trim(),
                        fail_count
                    );
                }
            }

            // Record time spent on request.
            let end_time = start_time.elapsed();
            info.tracker.num_requests.fetch_add(1, Ordering::SeqCst);
            let time_spent = end_time.as_millis() as usize;
            info.tracker
                .total_time_spent
                .fetch_add(time_spent, Ordering::SeqCst);

            // Put this part of the code in its own scope so that
            // we unlock the mutex as soon as we're done with it.
            // Otherwise, we'd have to wait until the sleep call
            // is done before the mutex is unlocked.
            {
                // Add the most recent request to the deque, removing the oldest if necessary.
                let mut recent_requests = info.tracker.recent_requests.lock().await;
                while recent_requests.len() >= MAX_RECENT_REQUESTS {
                    recent_requests.pop_front();
                }

                recent_requests.push_back(time_spent);
            }

            // Sleep between requests so we don't get ourselves banned by webreg
            tokio::time::sleep(Duration::from_secs_f64(info.cooldown)).await;
        }
    }

    // Out of loop, this should run only if we need to exit the scraper (e.g., need to log back in)
    #[cfg(feature = "scraper")]
    {
        if !writer.buffer().is_empty() {
            println!(
                "[{}] [{}] Buffer not empty! Buffer has length {}.",
                info.term,
                get_pretty_time(),
                writer.buffer().len()
            );
        }

        writer.flush().unwrap();
        // Debugging possible issues with the buffer
        println!(
            "[{}] [{}] Buffer flushed. Final buffer length: {}.",
            info.term,
            get_pretty_time(),
            writer.buffer().len()
        );
    }
}

#[cfg(feature = "scraper")]
struct CourseFile {
    /// The file containing data combined from *all* sections.
    overall_file: File,

    /// The file**s** containing data for each section family.
    section_files: HashMap<String, File>,
}

// in the form: (enrolled, available, waitlisted, total)
#[cfg(feature = "scraper")]
#[derive(Clone, Copy, Default)]
struct CourseStat(i64, i64, i64, i64);

#[cfg(feature = "scraper")]
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

#[cfg(feature = "scraper")]
impl AddAssign<CourseStat> for CourseStat {
    fn add_assign(&mut self, rhs: CourseStat) {
        self.0 += rhs.0;
        self.1 += rhs.1;
        self.2 += rhs.2;
        self.3 += rhs.3;
    }
}

#[cfg(feature = "scraper")]
impl Sum for CourseStat {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        // For our use case here, it is assumed that we have at least one element.
        iter.reduce(|prev, next| prev + next).unwrap_or_default()
    }
}
