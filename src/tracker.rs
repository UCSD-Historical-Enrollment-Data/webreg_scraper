use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::time::Instant;
use webweg::wrapper::input_types::{SearchRequestBuilder, SearchType};
use webweg::wrapper::WebRegWrapper;

use {
    crate::util::get_epoch_time,
    std::fs::OpenOptions,
    std::io::{BufWriter, Write},
    std::path::Path,
};

use crate::types::{TermInfo, WrapperState};
use crate::util::get_pretty_time;

const TIME_BETWEEN_WAIT_SEC: u64 = 3;
const MAX_NUM_REGISTER: usize = 25;
const MAX_NUM_FAILURES: usize = 50;
const MAX_RECENT_REQUESTS: usize = 2000;

/// Runs the WebReg tracker. This will optionally attempt to reconnect to
/// WebReg when signed out.
///
/// # Parameters
/// - `state`: The wrapper state.
/// - `wrapper_info`: The wrapper information.
/// - `verbose`: Whether the logging should be verbose.
pub async fn run_tracker(state: Arc<WrapperState>, wrapper_info: Arc<TermInfo>, verbose: bool) {
    try_login(&state).await;
    loop {
        state.is_running.store(true, Ordering::SeqCst);
        track_webreg_enrollment(&state, &wrapper_info, verbose).await;
        state.is_running.store(false, Ordering::SeqCst);

        if state.should_stop() {
            break;
        }

        if try_login(&state).await {
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
/// - `state`: The wrapper state.
/// - `info`: The term information.
/// - `verbose`: Whether logging should be verbose.
pub async fn track_webreg_enrollment(state: &Arc<WrapperState>, info: &TermInfo, verbose: bool) {
    let mut writer = {
        let file_name = format!(
            "enrollment_{}_{}.csv",
            chrono::offset::Local::now().format("%FT%H_%M_%S"),
            info.term.as_str()
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
        writer.flush().unwrap();
        let results = {
            let mut r = vec![];
            for search_query in &info.search_query {
                let mut temp = state
                    .wrapper
                    .req(info.term.as_str())
                    .parsed()
                    // TODO: Remove .clone usage here.
                    .search_courses(SearchType::Advanced(search_query.clone()))
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

        println!(
            "[{}] [{}] Found {} results successfully.",
            info.term,
            get_pretty_time(),
            results.len()
        );

        for r in results {
            if state.should_stop() {
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

            let res = state
                .wrapper
                .req(info.term.as_str())
                .parsed()
                .get_enrollment_count(r.subj_code.trim(), r.course_code.trim())
                .await;

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

pub async fn try_login(state: &Arc<WrapperState>) -> bool {
    let address = format!(
        "{}:{}",
        state.api_base_endpoint.address, state.api_base_endpoint.port
    );
    let mut num_failures = 0;

    while num_failures < MAX_NUM_FAILURES {
        tokio::time::sleep(Duration::from_secs(TIME_BETWEEN_WAIT_SEC)).await;

        if state.should_stop() {
            break;
        }

        let Ok(data) = state
            .client
            .get(format!("http://{address}/cookie"))
            .send()
            .await
        else {
            num_failures += 1;
            continue;
        };

        let Ok(text) = data.text().await else {
            num_failures += 1;
            continue;
        };

        let json: Value = serde_json::from_str(text.as_str()).unwrap_or_default();
        if !json["cookie"].is_string() {
            continue;
        }

        let cookies = json["cookie"].as_str().unwrap().to_string();

        // Update the cookies for the general wrapper, but also authenticate the cookies.
        // Remember, we're sharing the same cookies.
        if login_with_cookies(&state.wrapper, cookies.as_str(), state).await {
            return true;
        }

        num_failures += 1;
    }

    false
}

async fn login_with_cookies(
    wrapper: &WebRegWrapper,
    cookies: &str,
    state: &Arc<WrapperState>,
) -> bool {
    wrapper.set_cookies(cookies);

    let mut num_tries = 0;
    while num_tries < MAX_NUM_REGISTER {
        tokio::time::sleep(Duration::from_secs(TIME_BETWEEN_WAIT_SEC)).await;

        if wrapper.register_all_terms().await.is_err() {
            num_tries += 1;
            continue;
        };

        // to ensure that login was successful, try to get all courses and ensure those courses are not empty for all terms.
        let mut is_successful = true;
        for term in state.all_terms.keys() {
            let Ok(all_courses) = wrapper
                .req(term)
                .parsed()
                .search_courses(SearchType::Advanced(SearchRequestBuilder::new()))
                .await
            else {
                is_successful = false;
                break;
            };

            if all_courses.is_empty() {
                is_successful = false;
                break;
            }
        }

        if !is_successful {
            continue;
        }

        break;
    }

    num_tries < MAX_NUM_REGISTER
}
