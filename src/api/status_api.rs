#![cfg(feature = "api")]

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tracing::info;

use crate::api::util::api_get_general;
use crate::types::WrapperState;

/// An endpoint for checking the status of a specific term's scrapers.
///
/// # Usage
/// The endpoint should be called like so:
/// ```
/// /<term>
/// ```
pub async fn api_get_term_status(
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("Called with path {term}.");

    api_get_general(
        term.as_str(),
        move |term_info| async move {
            let status = term_info.is_running.load(Ordering::SeqCst);
            (StatusCode::OK, Json(json!({ "status": status }))).into_response()
        },
        s,
    )
    .await
}

/// An endpoint for checking the status of a specific term's scrapers.
///
/// # Usage
/// The endpoint should be called like so:
/// ```
/// /<term>/<stat_type>
/// ```
pub async fn api_get_login_script_stats(
    Path((term, stat_type)): Path<(String, String)>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("Called with path ({term}, {stat_type}).");

    if stat_type != "start" && stat_type != "history" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Use either 'start' or 'history' as the endpoint."
            })),
        )
            .into_response();
    }

    let client = s.client.clone();
    api_get_general(
        term.as_str(),
        move |term_info| async move {
            match client
                .get(format!(
                    "http://{}:{}/{}",
                    term_info.recovery.address, term_info.recovery.port, stat_type
                ))
                .send()
                .await
            {
                Ok(o) => (
                    StatusCode::OK,
                    o.text().await.unwrap_or_else(|_| {
                        match stat_type.as_str() {
                            "start" => "0",
                            "history" => "[]",
                            _ => "{}",
                        }
                        .to_string()
                    }),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": e.to_string()
                    })),
                )
                    .into_response(),
            }
        },
        s,
    )
    .await
}
