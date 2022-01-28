mod webreg;
use crate::webreg::webreg::{SearchRequestBuilder, WebRegWrapper};
use std::error::Error;
use std::time::Instant;

const COOKIE: &str = include_str!("../cookie.txt");

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let w = WebRegWrapper::new(COOKIE, "WI22");
    let valid = w.is_valid().await;
    println!("Is valid? {}", valid);

    if !valid {
        return Ok(());
    }

    println!("Account name: {}", w.get_account_name().await);
    println!();

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

    println!();

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

    Ok(())
}
