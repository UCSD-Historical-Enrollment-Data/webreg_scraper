//! All methods in this file make the assumption that the provided cookies from
//! the request header exist AND only has ASCII characters. This is enforced by
//! the middleware.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::header::COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tracing::info;
use webweg::types::EnrollmentStatus;
use webweg::wrapper::input_types::{AddType, ExplicitAddType};

use crate::server::types::{
    ApiErrorType, BodyAddInfo, BodyPlanAdd, BodyScheduleNameChange, BodySectionId,
    BodySectionScheduleNameId, RawParsedApiResp, RawQueryStr, ScheduleQueryStr,
};
use crate::server::util::{build_add_plan_object, build_add_section_object};
use crate::types::WrapperState;

#[tracing::instrument(level = "info", skip(s))]
pub async fn post_register_term(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("POST endpoint `register_term` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    s.c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .associate_term()
        .await
        .map_or_else(
            |e| ApiErrorType::from(e).into_response(),
            |_| StatusCode::NO_CONTENT.into_response(),
        )
}

/// A function which should be called when the `schedule` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_schedule(
    headers: HeaderMap,
    Query(schedule): Query<ScheduleQueryStr>,
    Query(req_type): Query<RawQueryStr>,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `schedule` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    let schedule_slice = schedule.name.as_deref();
    let builder = s.c_wrapper.req(term.as_str()).override_cookies(cookies);

    if req_type.raw.unwrap_or(false) {
        RawParsedApiResp::Raw(builder.raw().get_schedule(schedule_slice).await)
    } else {
        RawParsedApiResp::Parsed(builder.parsed().get_schedule(schedule_slice).await)
    }
    .into_response()
}

/// A function which should be called when the `schedule` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_schedule_list(
    headers: HeaderMap,
    Query(req_type): Query<RawQueryStr>,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `schedule_list` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    let builder = s.c_wrapper.req(term.as_str()).override_cookies(cookies);

    if req_type.raw.unwrap_or(false) {
        RawParsedApiResp::Raw(builder.raw().get_schedule_list().await)
    } else {
        RawParsedApiResp::Parsed(builder.parsed().get_schedule_list().await)
    }
    .into_response()
}

/// A function which should be called when the `events` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_events(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `events` called");
    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();

    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .get_events()
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `rename_schedule` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_rename_schedule(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodyScheduleNameChange>,
) -> Response {
    info!("POST endpoint `rename_schedule` called");
    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();

    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .rename_schedule(body.old_name, body.new_name)
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `validate_add_section` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_validate_add_section(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodyAddInfo>,
) -> Response {
    info!("POST endpoint `validate_add_section` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    let add_req = build_add_section_object(&body);
    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .validate_add_section(AddType::DecideForMe, &add_req)
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `add_section` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_add_section(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodyAddInfo>,
) -> Response {
    info!("POST endpoint `add_section` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    let add_req = build_add_section_object(&body);
    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .add_section(AddType::DecideForMe, add_req, body.validate.unwrap_or(true))
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `validate_add_plan` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_validate_add_plan(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodyPlanAdd>,
) -> Response {
    info!("POST endpoint `validate_add_plan` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    let plan_add = build_add_plan_object(&body);
    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .validate_add_to_plan(&plan_add)
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `add_plan` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_add_plan(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodyPlanAdd>,
) -> Response {
    info!("POST endpoint `add_plan` called");

    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();
    let plan_add = build_add_plan_object(&body);
    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .add_to_plan(plan_add, body.validate.unwrap_or(true))
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `remove_plan` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_remove_plan(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodySectionScheduleNameId>,
) -> Response {
    info!("POST endpoint `remove_plan` called");
    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();

    let req = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed()
        .remove_from_plan(body.section_id.as_str(), body.schedule_name.as_deref())
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}

/// A function which should be called when the `drop_section` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn post_drop_section(
    headers: HeaderMap,
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
    Json(body): Json<BodySectionId>,
) -> Response {
    info!("POST endpoint `drop_section` called");
    let cookies = headers.get(COOKIE).unwrap().to_str().unwrap();

    let requester = s
        .c_wrapper
        .req(term.as_str())
        .override_cookies(cookies)
        .parsed();

    let enroll_status = match requester.get_schedule(None).await {
        Ok(o) => {
            let sec = o
                .into_iter()
                .filter(|s| match s.enrolled_status {
                    EnrollmentStatus::Enrolled => true,
                    EnrollmentStatus::Waitlist { .. } => true,
                    EnrollmentStatus::Planned => false,
                    EnrollmentStatus::Unknown => false,
                })
                .find(|d| d.section_id == body.section_id.as_str());

            match sec {
                None => {
                    return ApiErrorType::from((
                        StatusCode::NOT_FOUND,
                        format!(
                            "You don't appeared to be enrolled in section {}",
                            body.section_id
                        ),
                        None,
                    ))
                    .into_response();
                }
                Some(s) => match s.enrolled_status {
                    EnrollmentStatus::Enrolled => ExplicitAddType::Enroll,
                    EnrollmentStatus::Waitlist { .. } => ExplicitAddType::Waitlist,
                    s => {
                        return ApiErrorType::from((
                            StatusCode::NOT_FOUND,
                            format!(
                                "You don't appeared to be enrolled in section {}",
                                body.section_id
                            ),
                            Some(format!("Your enrollment status: {:?}", s)),
                        ))
                        .into_response();
                    }
                },
            }
        }
        Err(err) => {
            return ApiErrorType::from(err).into_response();
        }
    };

    let req = requester
        .drop_section(enroll_status, body.section_id.as_str())
        .await;

    req.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |b| (StatusCode::OK, Json(json!({ "success": b }))).into_response(),
    )
}
