#![allow(dead_code)]
mod export;
mod schedule;
mod tracker;
mod util;
mod webreg;

use crate::export::exporter::save_schedules;
use crate::schedule::scheduler::{self, ScheduleConstraint};
use crate::webreg::webreg_wrapper::{
    CourseLevelFilter, PlanAdd, SearchRequestBuilder, WebRegWrapper,
};
use std::error::Error;
use std::time::{Duration, Instant};

// When I feel like everything's good enough, I'll probably make this into
// a better interface for general users.
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
                .add_subject("COGS")
                .add_subject("MATH")
                .add_subject("ECE")
                .filter_courses_by(CourseLevelFilter::LowerDivision)
                .filter_courses_by(CourseLevelFilter::UpperDivision),
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
    println!("{} enrollment count vs. {} sections parsed.", ct_a, ct_b);

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
    let search_res = w.search_courses_detailed(search).await.unwrap();

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
