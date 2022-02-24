//! I would use Cargo's test suite, but it's kind of hard to practically
//! test many of these functions, especially since the data that I would
//! need to test these functions with can change at any given time.

use crate::export::exporter::save_schedules;
use crate::schedule::scheduler::{self, ScheduleConstraint};
use crate::webreg::webreg_wrapper::{
    EnrollWaitAdd, PlanAdd, SearchRequestBuilder, SearchType, WebRegWrapper,
};
use std::time::{Duration, Instant};

/// Runs very basic tests.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
pub async fn run_basic_tests(w: &WebRegWrapper<'_>) {
    test_enroll_unenroll(w, false).await;
}

/// Attempts to enroll in a random section, and then unenroll after. This prints
/// the schedule out.
///
/// # Parameters
/// - `w`: The wrapper.
#[allow(dead_code)]
pub async fn test_enroll_unenroll(w: &WebRegWrapper<'_>, test_enroll: bool) {
    for c in w.get_schedule(None).await.unwrap() {
        println!("{}", c.to_string());
    }

    println!("==========================================");

    let course = w
        .search_courses_detailed(SearchType::BySection("079911"))
        .await
        .unwrap();

    if test_enroll {
        assert_eq!(1, course.len());
        println!(
            "Attempting to enroll in, or waitlist, {} => {:?}",
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
    } else {
        let (subj, crsc) = course[0].subj_course_id.split_once(" ").unwrap();
        println!(
            "Attempting to plan {} => {:?}",
            course[0].subj_course_id,
            w.add_to_plan(
                PlanAdd {
                    subject_code: subj,
                    course_code: crsc,
                    section_number: &course[0].section_id,
                    section_code: &course[0].section_code,
                    grading_option: Some("L"),
                    schedule_name: None,
                    unit_count: 4
                },
                true
            )
            .await
        );
    }

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
pub async fn section_search_filter(w: &WebRegWrapper<'_>) {
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
pub async fn section_parse(w: &WebRegWrapper<'_>) {
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
pub async fn get_schedules(
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
                .await
                .unwrap_or_else(|_| false);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }

            println!("{}", section.to_string());
        }
    }
}
