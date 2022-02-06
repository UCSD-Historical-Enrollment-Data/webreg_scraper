mod webreg;
mod tracker;
mod util;

use crate::webreg::webreg::{SearchRequestBuilder, WebRegWrapper};
use std::error::Error;
use std::time::Instant;

const COOKIE: &str = include_str!("../cookie.txt");

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let w = WebRegWrapper::new(COOKIE, "SP22");
    let valid = w.is_valid().await;

    if !valid {
        println!("Failed to login.");
        return Ok(());
    }

    println!("Logged in successfully. Account name: {}", w.get_account_name().await);
    println!();

    tracker::track::track_webreg_enrollment(
        &w,
        &SearchRequestBuilder::new()
            .add_subject("CSE")
            .add_subject("COGS")
    ).await;

    Ok(())
}

/// Performs a basic test of the `WebRegWrapper`.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
async fn basic_intro(w: &WebRegWrapper<'_>) -> () {
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