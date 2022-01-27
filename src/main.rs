mod webreg;
use crate::webreg::webreg::{SearchRequestBuilder, WebRegWrapper};
use std::error::Error;

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
    let search_res = w
        .search_courses(
            SearchRequestBuilder::new()
                .add_course("poli 28")
                .add_course("cse 130")
                .add_course("cogs 1")
                .add_course("math 183")
                .add_course("math 180a")
                .add_course("cse 8b"),
        )
        .await
        .unwrap();
    println!("Found {} courses!", search_res.len());
    for x in search_res {
        println!("{}", x.to_string());
    }

    Ok(())
}
