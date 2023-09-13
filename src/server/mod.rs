use std::sync::Arc;

use axum::routing::get;
use axum::{middleware as mw, Router};

use crate::server::endpoints::{status, ww_general};
use crate::server::middleware::{cookie_validator, running_validator, term_validator};
use crate::types::WrapperState;

mod endpoints;
mod middleware;

/// Creates a router that can be used by `axum`.
///
/// # Parameters
/// - `app_state`: The app server state.
///
/// # Returns
/// The router.
pub fn create_router(app_state: Arc<WrapperState>) -> Router {
    // Router whose endpoints require cookie header
    let cookie_router = Router::new().layer(mw::from_fn_with_state(
        app_state.clone(),
        cookie_validator::check_cookies,
    ));

    // General router
    let parsed_router = Router::new()
        .route("/course_info", get(ww_general::get_course_info))
        .route("/prerequisites", get(ww_general::get_prerequisites))
        .route("/search", get(ww_general::get_search_courses))
        .route("/department_codes", get(ww_general::get_department_codes))
        .route("/subject_codes", get(ww_general::get_subject_codes))
        .merge(cookie_router)
        .layer(mw::from_fn_with_state(
            app_state.clone(),
            term_validator::validate_term,
        ));

    // General router (no term)
    let general_router = Router::new().route("/terms", get(ww_general::get_all_terms));

    // Live WebReg router.
    let webreg_router = Router::new()
        .merge(parsed_router)
        .nest("/general", general_router)
        .layer(mw::from_fn_with_state(
            app_state.clone(),
            running_validator::validate_wrapper_running,
        ));

    Router::new()
        .route("/health", get(status::get_health))
        .nest("/live/:term", webreg_router)
        .route("/timing/:term", get(status::get_timing_stats))
        .route("/login_stat/:stat", get(status::get_login_script_stats))
        .with_state(app_state)
}
