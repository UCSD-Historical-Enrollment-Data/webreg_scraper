#![allow(dead_code)]
mod export;
mod schedule;
mod tracker;
mod util;

use crate::tracker::run_tracker;
use once_cell::sync::Lazy;
use rocket::response::content;
use rocket::{get, routes};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::webreg_wrapper::WebRegWrapper;

/// All terms and their associated "recovery URLs"
pub const TERMS: [[&str; 2]; 4] = [
    ["SP22", "http://localhost:3000/cookie"],
    ["S122", "http://localhost:3001/cookie"],
    ["S222", "http://localhost:3002/cookie"],
    ["S322", "http://localhost:3003/cookie"],
];

/// The cooldown, in seconds. The overall cooldown will be given by BASE_COOLDOWN * TERMS.len().
pub const BASE_COOLDOWN: f64 = 1.5;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct WebRegHandler<'a> {
    wrapper: Arc<Mutex<WebRegWrapper<'a>>>,
    terms_index: usize,
}

static WEBREG_WRAPPERS: Lazy<HashMap<&str, WebRegHandler>> = Lazy::new(|| {
    let mut map: HashMap<&str, WebRegHandler> = HashMap::new();

    for (i, [term, _]) in TERMS.iter().enumerate() {
        let cookie = get_cookies(term);
        let cookie = cookie.trim();
        map.insert(
            term,
            WebRegHandler {
                wrapper: Arc::new(Mutex::new(WebRegWrapper::new(cookie.to_string(), term))),
                terms_index: i,
            },
        );
    }

    map
});

// When I feel like everything's good enough, I'll probably make this into
// a better interface for general users.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("WebRegWrapper Version {}\n", VERSION);

    let mut handles = vec![];
    for (_, wg_handler) in WEBREG_WRAPPERS.iter() {
        let clone = wg_handler.wrapper.clone();
        handles.push(tokio::spawn(async move {
            run_tracker(
                clone,
                match TERMS[wg_handler.terms_index][1] {
                    x if x.is_empty() => None,
                    y => Some(y),
                },
                TERMS[wg_handler.terms_index][0],
            )
            .await;
        }));

        tokio::time::sleep(Duration::from_secs_f64(BASE_COOLDOWN)).await;
    }

    rocket::build()
        .mount("/", routes![get_course_info])
        .launch()
        .await
        .unwrap();

    Ok(())
}

#[get("/course/<term>/<subj>/<num>")]
async fn get_course_info(term: String, subj: String, num: String) -> content::Json<String> {
    if let Some(wg_handler) = WEBREG_WRAPPERS.get(&term.as_str()) {
        let wg_handler = wg_handler.wrapper.lock().await;
        let res = wg_handler.get_course_info(&subj, &num).await;
        drop(wg_handler);
        match res {
            Ok(o) => content::Json(serde_json::to_string(&o).unwrap_or_else(|_| "[]".to_string())),
            Err(e) => content::Json(json!({ "error": e }).to_string()),
        }
    } else {
        content::Json(
            json!({
                "error": "Invalid term specified."
            })
            .to_string(),
        )
    }
}

fn get_cookies(term: &str) -> String {
    let file_name = format!("cookie_{}.txt", term);
    let file = Path::new(&file_name);
    if !file.exists() {
        eprintln!("'{}' file does not exist. Try again.", file_name);
        return "".to_string();
    }

    fs::read_to_string(file).unwrap_or_else(|_| "".to_string())
}
