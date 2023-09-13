use std::sync::Arc;

use crate::server::types::{
    ApiErrorType, BodySearchType, CourseQueryStr, RawParsedApiResp, RawQueryStr,
};
use crate::types::WrapperState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::log::info;

/// A function which should be called when the `terms` endpoint from the `general`
/// route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_all_terms(State(s): State<Arc<WrapperState>>) -> Response {
    info!("GET endpoint `terms` called");
    s.wrapper.get_all_terms().await.map_or_else(
        |e| ApiErrorType::from(e).into_response(),
        |t| (StatusCode::OK, Json(t)).into_response(),
    )
}

/// A function which should be called when the `course_info` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_course_info(
    Path(term): Path<String>,
    Query(crsc): Query<CourseQueryStr>,
    Query(req_type): Query<RawQueryStr>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `course_info` called");
    let builder = s.wrapper.req(term.as_str());
    if req_type.raw.unwrap_or(false) {
        RawParsedApiResp::Raw(
            builder
                .raw()
                .get_course_info(crsc.subject, crsc.number)
                .await,
        )
    } else {
        RawParsedApiResp::Parsed(
            builder
                .parsed()
                .get_course_info(crsc.subject, crsc.number)
                .await,
        )
    }
    .into_response()
}

/// A function which should be called when the `prerequisites` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_prerequisites(
    Path(term): Path<String>,
    Query(crsc): Query<CourseQueryStr>,
    Query(req_type): Query<RawQueryStr>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `prerequisites` called");

    let builder = s.wrapper.req(term.as_str());
    if req_type.raw.unwrap_or(false) {
        RawParsedApiResp::Raw(
            builder
                .raw()
                .get_prerequisites(crsc.subject, crsc.number)
                .await,
        )
    } else {
        RawParsedApiResp::Parsed(
            builder
                .parsed()
                .get_prerequisites(crsc.subject, crsc.number)
                .await,
        )
    }
    .into_response()
}

/// A function which should be called when the `search_courses` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_search_courses(
    Path(term): Path<String>,
    Query(req_type): Query<RawQueryStr>,
    State(s): State<Arc<WrapperState>>,
    // The Json needs to be the last parameter since its request body is being consumed.
    Json(search_info): Json<BodySearchType>,
) -> Response {
    info!("GET endpoint `search` called");

    let builder = s.wrapper.req(term.as_str());
    if req_type.raw.unwrap_or(false) {
        RawParsedApiResp::Raw(builder.raw().search_courses(search_info.into()).await)
    } else {
        RawParsedApiResp::Parsed(builder.parsed().search_courses(search_info.into()).await)
    }
    .into_response()
}

/// A function which should be called when the `subject_codes` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_subject_codes(
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `subject_codes` called");
    let req = s
        .wrapper
        .req(term.as_str())
        .parsed()
        .get_subject_codes()
        .await;

    match req {
        Ok(o) => (StatusCode::OK, Json(o)).into_response(),
        Err(e) => ApiErrorType::from(e).into_response(),
    }
}

/// A function which should be called when the `department_codes` endpoint from the
/// `parsed` route is called.
#[tracing::instrument(level = "info", skip(s))]
pub async fn get_department_codes(
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("GET endpoint `department_codes` called");
    let req = s
        .wrapper
        .req(term.as_str())
        .parsed()
        .get_department_codes()
        .await;

    match req {
        Ok(o) => (StatusCode::OK, Json(o)).into_response(),
        Err(e) => ApiErrorType::from(e).into_response(),
    }
}
