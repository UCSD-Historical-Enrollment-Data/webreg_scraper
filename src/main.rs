#![allow(dead_code)]
mod export;
mod schedule;
mod tests;
mod tracker;
mod util;
mod webreg;

use crate::tracker::run_tracker;
use crate::webreg::webreg_wrapper::{SearchRequestBuilder, WebRegWrapper};
use std::error::Error;

const TERM: &str = "SP22";
const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        eprintln!("Failed to login.");
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
