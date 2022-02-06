mod tracker;
mod util;
mod webreg;

use crate::webreg::webreg::{SearchRequestBuilder, WebRegWrapper};
use std::error::Error;
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let cookie = get_cookies();
    let cookie = cookie.trim();
    if cookie.is_empty() {
        eprintln!("'cookie.txt' file is empty. Try again.");
        return Ok(());
    }

    let w = WebRegWrapper::new(cookie, "SP22");

    let valid = w.is_valid().await;

    if !valid {
        println!("Failed to login.");
        return Ok(());
    }

    println!(
        "Logged in successfully. Account name: {}",
        w.get_account_name().await
    );
    println!();

    tracker::track::track_webreg_enrollment(
        &w,
        &SearchRequestBuilder::new()
            .add_subject("CSE")
            .add_subject("COGS"),
    )
    .await;

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
        return "".to_string()
    }

    fs::read_to_string(file).unwrap_or_else(|_| "".to_string())
}


/// Performs a basic test of the `WebRegWrapper`.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
async fn basic_intro(w: &WebRegWrapper<'_>) {
    // Get my schedule
    let my_schedule = w.get_schedule(None).await.unwrap();
    println!(
        "Taking {} courses for a total of {} unit(s)",
        my_schedule.len(),
        my_schedule.iter().map(|x| x.units).sum::<f32>()
    );

    for d in my_schedule {
        println!("{}", d.to_string());
    }

    // Get CSE 100 courses
    let courses = w.get_course_info("CSE", "100").await.unwrap();

    println!("{} possible sections found.", courses.len());
    for d in courses {
        println!("{}", d.to_string())
    }

    // Search stuff.
    let start = Instant::now();
    let search_res = w
        .search_courses_detailed(
            SearchRequestBuilder::new()
                .add_course("MATH 20B")
                .add_course("MATH 10B"),
        )
        .await
        .unwrap();

    let duration = start.elapsed();

    println!(
        "Found {} sections in {} seconds!",
        search_res.len(),
        duration.as_secs_f32()
    );
}
