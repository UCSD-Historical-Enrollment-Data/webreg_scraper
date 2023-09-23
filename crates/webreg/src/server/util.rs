use crate::server::types::{BodyAddInfo, BodyPlanAdd};
use webweg::wrapper::input_types::{EnrollWaitAdd, GradeOption, PlanAdd};

/// A helper function to automatically convert the given grading option and unit count from
/// a request body to something that the library can use.
///
/// # Parameters
/// - `grading_option`: The grading option.
/// - `unit_count`: The unit count.
///
/// # Returns
/// The "parsed" version that can be used by the library.
pub fn parse_grade_option_unit_count(
    grading_option: &Option<String>,
    unit_count: Option<i64>,
) -> (GradeOption, Option<u8>) {
    let grading_option = match grading_option {
        Some(g) => match g.as_str() {
            "L" | "l" => GradeOption::L,
            "P" | "p" => GradeOption::P,
            "S" | "s" => GradeOption::S,
            _ => GradeOption::L,
        },
        None => GradeOption::L,
    };

    let unit_count = unit_count.and_then(|d| u8::try_from(d).ok());

    (grading_option, unit_count)
}

/// Builds the `PlanAdd` object that can be used for the library.
///
/// # Parameters
/// - `body`: The body from the request.
///
/// # Returns
/// The `PlanAdd` object.
pub fn build_add_plan_object(body: &BodyPlanAdd) -> PlanAdd {
    let (grading_option, unit_count) =
        parse_grade_option_unit_count(&body.grading_option, Some(body.unit_count));

    let mut plan_add = PlanAdd::builder()
        .with_subject_code(body.subject_code.as_str())
        .with_course_code(body.course_code.as_str())
        .with_section_id(body.section_id.as_str())
        .with_section_code(body.section_code.as_str())
        .with_grading_option(grading_option)
        .with_unit_count(unit_count.unwrap_or(4));

    if let Some(ref s) = body.schedule_name {
        plan_add = plan_add.with_schedule_name(s);
    }

    plan_add.try_build().unwrap()
}

/// Builds the `EnrollWaitAdd` object that can be used for the library.
///
/// # Parameters
/// - `body`: The body from the request.
///
/// # Returns
/// The `EnrollWaitAdd` object.
pub fn build_add_section_object(body: &BodyAddInfo) -> EnrollWaitAdd {
    let (grading_option, unit_count) =
        parse_grade_option_unit_count(&body.grading_option, body.unit_count);

    let mut add_req = EnrollWaitAdd::builder()
        .with_section_id(body.section_id.as_str())
        .with_grading_option(grading_option);

    if let Some(u) = unit_count {
        add_req = add_req.with_unit_count(u);
    }

    add_req.try_build().unwrap()
}
