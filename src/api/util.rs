use std::future::Future;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;
use webweg::wrapper::WrapperError;

use crate::types::{TermInfo, WrapperState};

/// A helper function that automatically handles the case when an invalid term is given.
///
/// # Parameters
/// - `term`: The possibly invalid term.
/// - `res`: The closure that produces the value to be returned by the API.
/// - `state`: The wrapper's state.
///
/// # Returns
/// The response.
#[inline]
pub async fn api_get_general<A, U>(term: &str, res: A, state: Arc<WrapperState>) -> Response
where
    A: FnOnce(Arc<TermInfo>) -> U,
    U: Future<Output = Response>,
{
    if let Some(term_data) = state.all_wrappers.get(term) {
        res(term_data.clone()).await
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Specified term is not supported at this time."
            })),
        )
            .into_response()
    }
}

/// Processes the return type from the WebReg wrapper into a Response type for
/// the API wrapper.
///
/// # Parameters
/// - `res`: The result of the call to the wrapper.
///
/// # Returns
/// The response.
pub fn process_return<T>(res: Result<T, WrapperError>) -> Response
where
    T: Serialize,
{
    match res {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": err.to_string()
            })),
        )
            .into_response(),
    }
}
