#![allow(dead_code)]
mod export;
mod schedule;
mod tracker;
mod util;
mod webreg;

use reqwest::Client;
use serde_json::Value;

use crate::export::exporter::save_schedules;
use crate::schedule::scheduler::{self, ScheduleConstraint};
use crate::util::get_pretty_time;
use crate::webreg::webreg_wrapper::{
    CourseLevelFilter, EnrollWaitAdd, PlanAdd, SearchRequestBuilder, SearchType, WebRegWrapper,
};
use std::error::Error;
use std::time::{Duration, Instant};

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
        run_basic_tests(&w).await;
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

/// Runs very basic tests.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
async fn run_basic_tests(w: &WebRegWrapper<'_>) {
    for c in w.get_schedule(None).await.unwrap() {
        println!("{}", c.to_string());
    }
    /*
    for c in w.get_course_info("CSE", "127").await.unwrap() {
        println!("{}", c.to_string());
    } */
}

/// Attempts to enroll in a random section, and then unenroll after. This prints
/// the schedule out.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
async fn test_enroll_unenroll(w: &WebRegWrapper<'_>) {
    for c in w.get_schedule(None).await.unwrap() {
        println!("{}", c.to_string());
    }

    println!("==========================================");

    let course = w
        .search_courses_detailed(SearchType::BySection("079588"))
        .await
        .unwrap();
    assert_eq!(1, course.len());
    println!(
        "Attempting to enroll in, or waitlist, {} => {}",
        course[0].subj_course_id,
        w.add_section(
            course[0].available_seats > 0,
            EnrollWaitAdd {
                section_number: &course[0].section_id,
                grading_option: None,
                unit_count: None,
            },
            true
        )
        .await
    );

    println!("==========================================");

    for c in w.get_schedule(None).await.unwrap() {
        println!("{}", c.to_string());
    }
}

/// Tests the section filter functionality.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
async fn section_search_filter(w: &WebRegWrapper<'_>) {
    // Test filtering specific sections from different departments
    if let Some(r) = w
        .search_courses_detailed(SearchType::ByMultipleSections(&[
            "079913", "078616", "075219",
        ]))
        .await
    {
        for c in r {
            println!("{}", c.to_string());
        }
    }

    println!("=============================");
    // Test general search
    if let Some(r) = w
        .search_courses_detailed(SearchType::Advanced(
            &SearchRequestBuilder::new().add_course("MATH 154"),
        ))
        .await
    {
        for c in r {
            println!("{}", c.to_string());
        }
    }

    println!("=============================");
}

/// Compares the number of sections that can be enrolled to the number
/// of sections that were parsed successfully.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
async fn section_parse(w: &WebRegWrapper<'_>) {
    const SUBJECT_CODE: &str = "MAE";
    const COURSE_CODE: &str = "30B";
    // Search stuff.
    let enrollment_count_vec = w
        .get_enrollment_count(SUBJECT_CODE, COURSE_CODE)
        .await
        .unwrap();
    let ct_a = enrollment_count_vec.len();
    for c in enrollment_count_vec {
        println!("{}", c.to_string().trim());
    }

    println!("=============================");
    let course_info_vec = w.get_course_info(SUBJECT_CODE, COURSE_CODE).await.unwrap();
    let ct_b = course_info_vec.len();
    for c in course_info_vec {
        println!("{}", c.to_string());
    }

    println!("=============================");
    println!(
        "{} sections that can be enrolled vs. {} sections parsed.",
        ct_a, ct_b
    );

    println!("=============================");
    let schedule = w.get_schedule(Some("Test")).await.unwrap();
    for s in schedule {
        println!("{}", s.to_string());
    }
}

/// Gets possible schedules, optionally adding them to WebReg.
///
/// # Parameters
/// - `w`: The `WebRegWrapper`.
/// - `classes`: All classes to check.
/// - `add_to_webreg`: Whether to add your schedules to WebReg.
/// - `print`: Whether to print the schedules (set to `false` if you don't need to see the schedules)
/// - `save_to_file`: Whether to save your schedules to a file. If this is selected, the other options are ignored.
async fn get_schedules(
    w: &WebRegWrapper<'_>,
    classes: &[&str],
    add_to_webreg: bool,
    print: bool,
    save_to_file: bool,
) {
    if classes.is_empty() {
        return;
    }

    let mut search = SearchRequestBuilder::new();
    for c in classes {
        search = search.add_course(c);
    }
    let search_res = w
        .search_courses_detailed(SearchType::Advanced(&search))
        .await
        .unwrap();

    println!("Found {} sections!", search_res.len());
    if print {
        for s in &search_res {
            println!("{}", s.to_string());
        }
    }

    let dur = Instant::now();
    let schedules = scheduler::generate_schedules(classes, &search_res, ScheduleConstraint::new());

    println!(
        "{} schedules found in {} seconds.",
        schedules.len(),
        dur.elapsed().as_secs_f32()
    );

    if save_to_file {
        save_schedules(&schedules);
        return;
    }

    if !add_to_webreg && !print {
        return;
    }

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

        for section in schedule.sections {
            if add_to_webreg {
                let (sub, code) = section.subj_course_id.split_once(" ").unwrap();
                // TODO add_to_plan doesn't seem to work fully (see CSE 130)
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

            println!("{}", section.to_string());
        }
    }
}
