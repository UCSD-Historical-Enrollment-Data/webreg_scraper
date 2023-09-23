use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::time::Instant;
use tracing::log::error;
use tracing::{info, warn};
use webweg::wrapper::input_types::{SearchRequestBuilder, SearchType};

use crate::scraper::util::get_epoch_time;
use crate::types::{TermInfo, WrapperState};
use {
    std::fs::OpenOptions,
    std::io::{BufWriter, Write},
    std::path::Path,
};

/// The number of times we should allow consecutive failure requests before attempting to get
/// new session cookies.
const MAX_NUM_SEARCH_REQUESTS: usize = 12;
/// The number of times we should attempt to get new session cookies.
const MAX_NUM_LOGIN_FAILURES: i32 = 30;
/// The number of times we should attempt to register the session cookies.
const MAX_NUM_REGISTER: usize = 25;
/// The base delay when getting new session cookies. Note that, when attempting to get new
/// session cookies, we want to use exponential backoff to ensure that if we can't get cookies
/// the first time, we wait a bit longer before trying again.
const BASE_DELAY_FOR_SESSION_COOKIE: f64 = 8.0;
/// The general delay, i.e., the delay between making requests.
const GENERAL_DELAY: u64 = 3;

/// Runs the WebReg tracker. This will optionally attempt to reconnect to
/// WebReg when signed out.
///
/// # Parameters
/// - `state`: The wrapper state.
/// - `verbose`: Whether the logging should be verbose.
pub async fn run_tracker(state: Arc<WrapperState>, verbose: bool) {
    if !try_login(&state).await {
        error!("Initial login could not be completed, so the tracker will no longer run.");
        return;
    }

    loop {
        state.is_running.store(true, Ordering::SeqCst);

        let current_loop_stop_flag = Arc::new(AtomicBool::new(false));
        let mut futures = FuturesUnordered::new();
        for term_data in state.all_terms.values() {
            futures.push(track_webreg_enrollment(
                &state,
                term_data,
                verbose,
                current_loop_stop_flag.clone(),
            ));
        }

        // Wait until ONE of the futures completed, indicating that ONE of the
        // runners is now done.
        futures.next().await;
        info!("A tracker is currently done. Attempting to stop other trackers.");
        current_loop_stop_flag.store(true, Ordering::SeqCst);
        while let Some(()) = futures.next().await {
            // Do nothing.
        }
        state.is_running.store(false, Ordering::SeqCst);

        info!("All trackers have been stopped.");
        if state.should_stop() {
            break;
        }

        // Attempt to login again.
        if try_login(&state).await {
            continue;
        }

        // Otherwise, gracefully quit.
        break;
    }

    // This should only run if we're 100% done with this
    // wrapper. For example, either the wrapper could not
    // log back in or we forced it to stop.
    info!("Quitting the tracker.");
}

/// Tracks WebReg for enrollment information. This will continuously check specific courses for
/// their enrollment information (number of students waitlisted/enrolled, total seats) along with
/// basic course information and store this in a CSV file for later processing.
///
/// # Parameters
/// - `state`: The wrapper state.
/// - `info`: The term information.
/// - `verbose`: Whether logging should be verbose.
/// - `current_loop_stop_flag`: Whether to stop any further requests for this function call
///                             instance.
async fn track_webreg_enrollment(
    state: &Arc<WrapperState>,
    info: &TermInfo,
    verbose: bool,
    current_loop_stop_flag: Arc<AtomicBool>,
) {
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
            warn!("[{}] No courses found. Exiting.", info.term);
            break;
        }

        info!(
            "[{}] Found {} results successfully.",
            info.term,
            results.len()
        );

        for r in results {
            // If the stop flag is set so that the scraper itself should STOP, or we just need
            // to stop for this iteration, then break out
            if state.should_stop() || current_loop_stop_flag.load(Ordering::SeqCst) {
                break 'main;
            }

            if fail_count != 0 && fail_count > MAX_NUM_SEARCH_REQUESTS {
                warn!(
                    "[{}] Too many failures when trying to request data from WebReg.",
                    info.term
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
                    warn!(
                        "[{}] An error occurred ({}). Skipping. (FAIL_COUNT: {})",
                        info.term, e, fail_count
                    );
                }
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    if verbose {
                        info!(
                            "[{}] Processing {} section(s) for {}",
                            info.term,
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
                    warn!(
                        "[{}] Course {} {} not found. Were you logged out? (FAIL_COUNT: {}).",
                        info.term,
                        r.subj_code.trim(),
                        r.course_code.trim(),
                        fail_count
                    );
                }
            }

            // Record time spent on request.
            let end_time = start_time.elapsed();
            info.tracker.add_stat(end_time.as_millis() as usize);

            // Sleep between requests so we don't get ourselves banned by webreg
            tokio::time::sleep(Duration::from_secs_f64(info.cooldown)).await;
        }
    }

    // Out of loop, this should run only if we need to exit the scraper (e.g., need to log back in)
    if !writer.buffer().is_empty() {
        info!(
            "[{}] Buffer not empty! Buffer has length {}.",
            info.term,
            writer.buffer().len()
        );
    }

    writer.flush().unwrap();
    // Debugging possible issues with the buffer
    info!(
        "[{}] Buffer flushed. Final buffer length: {}.",
        info.term,
        writer.buffer().len()
    );
}

/// Attempts to run the login script to get new session cookies, and then ensures that the
/// cookies themselves are valid.
///
/// # Parameters
/// - `state`: The wrapper state.
///
/// # Returns
/// `true` if the login process is successful, indicating that the wrapper is ready to
/// make requests again. `false` otherwise.
async fn try_login(state: &Arc<WrapperState>) -> bool {
    info!("Attempting to get new WebReg session cookies.");
    let address = format!(
        "{}:{}",
        state.cookie_server.address, state.cookie_server.port
    );

    let mut num_failures = 0;
    while num_failures <= MAX_NUM_LOGIN_FAILURES {
        let delay_time = 1.2_f64.powi(num_failures) * BASE_DELAY_FOR_SESSION_COOKIE;
        info!(
            "Waiting {delay_time} seconds before making request for new cookies ({num_failures}/{MAX_NUM_LOGIN_FAILURES})."
        );
        tokio::time::sleep(Duration::from_secs_f64(delay_time)).await;

        if state.should_stop() {
            warn!("Application state indicates that the process should stop, stopping.");
            break;
        }

        info!("Making a request to the cookie server (http://{address}/cookie) to get session cookies.");
        let data = match state
            .client
            .get(format!("http://{address}/cookie"))
            .send()
            .await
        {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to connect to the cookie server; reason: '{e}'");
                num_failures += 1;
                continue;
            }
        };

        let Ok(text) = data.text().await else {
            warn!("An unknown error occurred when making a request to the cookie server.");
            num_failures += 1;
            continue;
        };

        let json: Value = serde_json::from_str(text.as_str()).unwrap_or_default();
        info!("Received response from cookie server: '{json}'");
        if !json["cookie"].is_string() {
            warn!("The 'cookie' key from the response is not valid.");
            continue;
        }

        let cookies = json["cookie"].as_str().unwrap().to_string();

        // Update the cookies for the general wrapper, but also authenticate the cookies.
        // Remember, we're sharing the same cookies.
        if login_with_cookies(state, cookies.as_str()).await {
            info!("Cookies were successfully fetched and authenticated for all terms specified.");
            return true;
        }

        warn!("An unknown error occurred when trying to authenticate the cookies.");
        num_failures += 1;
    }

    false
}

/// Sets the cookies to the specified wrapper and then attempts to validate that the
/// cookies are valid. This will attempt to make several requests until either one
/// request is successful or all requests fail.
///
/// # Parameters
/// - `state`: The wrapper state.
/// - `cookies`: The session cookies to use.
///
/// # Returns
/// `true` if the login process is successful, indicating that the wrapper is ready to
/// make requests again. `false` otherwise.
#[inline]
async fn login_with_cookies(state: &Arc<WrapperState>, cookies: &str) -> bool {
    state.wrapper.set_cookies(cookies);

    let mut num_tries = 0;
    while num_tries <= MAX_NUM_REGISTER {
        tokio::time::sleep(Duration::from_secs(GENERAL_DELAY)).await;

        info!("Attempting to register all terms for the given session cookies.");
        if let Err(e) = state.wrapper.register_all_terms().await {
            num_tries += 1;
            warn!(
                "An error occurred when trying to register all terms ({num_tries}/{MAX_NUM_REGISTER}): '{e}'"
            );
            continue;
        };

        info!(
            "All terms for the cookies were registered. Now, checking that requests can be made."
        );
        // To ensure that login was successful, try to get all courses and ensure those courses
        // are not empty for all terms.
        let mut is_successful = true;
        for term in state.all_terms.keys() {
            let all_courses = match state
                .wrapper
                .req(term)
                .parsed()
                .search_courses(SearchType::Advanced(SearchRequestBuilder::new()))
                .await
            {
                Ok(o) => {
                    info!("Found {} courses for the term '{term}'.", o.len());
                    o
                }
                Err(e) => {
                    num_tries += 1;
                    warn!("Failed to fetch courses for term '{term}' ({num_tries}/{MAX_NUM_REGISTER}); error received: '{e}'");
                    is_successful = false;
                    break;
                }
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
