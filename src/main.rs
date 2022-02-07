mod schedule;
mod tracker;
mod util;
mod webreg;

use crate::schedule::scheduler;
use crate::webreg::webreg_wrapper::{PlanAdd, SearchRequestBuilder, WebRegWrapper};
use std::error::Error;
use std::time::{Duration};

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

    if cfg!(debug_assertions) {
        basic_intro(&w).await;
    } else {
        tracker::track::track_webreg_enrollment(
            &w,
            &SearchRequestBuilder::new()
                .add_subject("CSE")
                .add_subject("COGS"),
        )
        .await;
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

    // Get my schedules.
    println!("Schedules: {:?}", w.get_schedules().await.unwrap());

    // Search stuff.
    get_schedules(w, &["MATH 184", "CSE 30"], false).await;
}

/// Gets possible schedules, optionally adding them to WebReg.
///
/// # Parameters
/// - `w`: The `WebRegWrapper`.
/// - `classes`: All classes to check.
/// - `add_to_webreg`: Whether to add your schedules to WebReg.
async fn get_schedules(w: &WebRegWrapper<'_>, classes: &[&str], add_to_webreg: bool) {
    if classes.is_empty() {
        return; 
    }

    let mut search = SearchRequestBuilder::new();
    for c in classes {
        search = search.add_course(c);
    }
    let search_res = w.search_courses_detailed(search).await.unwrap();

    println!("Found {} sections! Results are:", search_res.len());
    for s in &search_res {
        println!("{}", s.to_string());
    }

    println!("\n");
    let schedules = scheduler::generate_schedules(
        &["MATH 180A", "POLI 28", "CSE 130", "HISC 108"],
        &search_res,
    );

    println!("{} schedules found.", schedules.len());
    let mut i = 0;
    for schedule in schedules {
        i += 1;
        let schedule_name = format!("My Schedule {}", i);
        println!(
            "{}",
            if add_to_webreg {
                format!("Adding '{}' to WebReg", schedule_name)
            } else {
                schedule_name.to_string()
            }
        );

        for (_, section) in schedule.sections {
            if add_to_webreg {
                let (sub, code) = section.subj_course_id.split_once(" ").unwrap();
                w.add_to_plan(
                    PlanAdd {
                        subject_code: sub,
                        course_code: code,
                        section_number: &*section.section_id,
                        section_code: &*section.section_code,
                        grading_option: None,
                        schedule_name: Some(&*schedule_name),
                        unit_count: 4,
                    },
                    true,
                )
                .await;
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
    }
}
