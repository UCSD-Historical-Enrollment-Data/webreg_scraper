use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use tracing::log::info;

use crate::types::WrapperState;

/// A function to be executed when the `health` endpoint is called.
#[tracing::instrument(skip(s))]
pub async fn get_health(State(s): State<Arc<WrapperState>>) -> Response {
    info!("Called `health` endpoint.");
    let status = s.is_running();
    let response = json!({ "api": status });

    info!("Returned status: {status}");
    (StatusCode::OK, Json(response)).into_response()
}

/// An endpoint for checking the time stats for a specific term's scrapers.
#[tracing::instrument(skip(s))]
pub async fn get_timing_stats(
    Path(term): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("Called with path '{term}'.");
    if let Some(t) = s.all_terms.get(term.as_str()) {
        let num_requests = t.tracker.num_requests.load(Ordering::SeqCst);
        let time_spent = t.tracker.total_time_spent.load(Ordering::SeqCst);
        let recent_requests = {
            let temp = t.tracker.recent_requests.lock().unwrap();
            temp.iter().copied().collect::<Vec<_>>()
        };

        let json = json!({
            "ttl_requests": num_requests,
            "ttl_time_ms": time_spent,
            "recent_requests": recent_requests
        });

        (StatusCode::OK, Json(json)).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// An endpoint for checking the status of a specific term's scrapers.
#[tracing::instrument(skip(s))]
pub async fn get_login_script_stats(
    Path(stat_type): Path<String>,
    State(s): State<Arc<WrapperState>>,
) -> Response {
    info!("Called with path '{stat_type}'.");

    if stat_type != "start" && stat_type != "history" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Use either 'start' or 'history' as the endpoint."
            })),
        )
            .into_response();
    }

    let cookie_url = format!(
        "http://{}:{}/{}",
        s.cookie_server.address, s.cookie_server.port, stat_type
    );

    match s.client.get(cookie_url).send().await {
        Ok(r) => {
            let resp = r.text().await.unwrap_or_else(|_| {
                match stat_type.as_str() {
                    "start" => "0",
                    "history" => "[]",
                    _ => "{}",
                }
                    .to_string()
            });

            // resp is a String, so if we were to just return Json(resp),
            // then we will end up with a string response even though we want
            // to return a JSON object. So, convert to Value first and *then*
            // return that as JSON.
            match serde_json::from_str::<Value>(resp.as_str()) {
                Ok(o) => (StatusCode::OK, Json(o)).into_response(),
                Err(e) => {
                    let err = json!({
                        "error": e.to_string()
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
                }
            }
        }
        Err(e) => {
            let json = json!({
                "error": e.to_string()
            });

            (StatusCode::INTERNAL_SERVER_ERROR, Json(json)).into_response()
        }
    }
}
