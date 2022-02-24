#![allow(dead_code)]
mod export;
mod schedule;
mod tests;
mod tracker;
mod util;
mod webreg;

use crate::util::get_pretty_time;
use reqwest::Client;
use serde_json::Value;

use crate::webreg::webreg_wrapper::{CourseLevelFilter, SearchRequestBuilder, WebRegWrapper};
use std::error::Error;
use std::time::Duration;

const TERM: &str = "SP22";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(debug_assertions)]
const TIMEOUT: [u64; 3] = [5, 10, 15];

#[cfg(not(debug_assertions))]
// The idea is that it should take no more than 15 minutes for
// WebReg to be available.
const TIMEOUT: [u64; 3] = [8 * 60, 6 * 60, 4 * 60];

// When I feel like everything's good enough, I'll probably make this into
// a better interface for general users.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("WebRegWrapper Version {}\n", VERSION);
    let cookie = get_cookies();
    let cookie = cookie.trim();
    if cookie.is_empty() {
        eprintln!("'cookie.txt' file is empty. Try again.");
        return Ok(());
    }

    let w = WebRegWrapper::new(cookie.to_string(), TERM);
    if !w.is_valid().await {
        println!("Failed to login.");
        return Ok(());
    }

    println!(
        "Logged in successfully. Account name: {}",
        w.get_account_name().await
    );

    if cfg!(debug_assertions) {
        tests::run_basic_tests(&w).await;
    } else {
        run_tracker(w, Some("http://localhost:3000/cookie")).await;
    }

    Ok(())
}

#[cfg(debug_assertions)]
fn get_cookies() -> String {
    include_str!("../cookie.txt").to_string()
}

#[cfg(not(debug_assertions))]
fn get_cookies() -> String {
    use std::fs;
    use std::path::Path;
    let file = Path::new("cookie.txt");
    if !file.exists() {
        eprintln!("'cookie.txt' file does not exist. Try again.");
        return "".to_string();
    }

    fs::read_to_string(file).unwrap_or_else(|_| "".to_string())
}

/// Runs the WebReg tracker. This will optionally attempt to reconnect to
/// WebReg when signed out.
///
/// # Parameters
/// - `w`: The wrapper.
/// - `cookie_url`: The URL to the API where new cookies can be requested. If none
/// is specified, then this will automatically terminate upon any issue with the
/// tracker.
async fn run_tracker(w: WebRegWrapper<'_>, cookie_url: Option<&str>) {
    let client = Client::new();

    let mut webreg_wrapper = w;
    loop {
        tracker::track_webreg_enrollment(
            &webreg_wrapper,
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
                match client.get(cookie_url.unwrap()).send().await {
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

                webreg_wrapper = WebRegWrapper::new(c, TERM);
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
