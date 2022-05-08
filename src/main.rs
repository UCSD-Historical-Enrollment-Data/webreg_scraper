#![allow(dead_code)]
mod export;
mod schedule;
mod tracker;
mod util;

use crate::tracker::run_tracker;
use once_cell::sync::Lazy;
use rocket::response::content;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{get, post, routes};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::webreg_wrapper::{Output, SearchRequestBuilder, SearchType, WebRegWrapper};

/// All terms and their associated "recovery URLs"
pub const TERMS: [[&str; 2]; 3] = [
    ["S122", "http://localhost:3001/cookie"],
    ["S222", "http://localhost:3002/cookie"],
    ["S322", "http://localhost:3003/cookie"],
];

/// The cooldown, in seconds. The overall cooldown will be given by BASE_COOLDOWN * TERMS.len().
pub const BASE_COOLDOWN: f64 = 1.5;
pub const RESET_COOLDOWN: f64 = 6.0;

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

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct SearchQuery {
    subjects: Vec<String>,
    courses: Vec<String>,
    departments: Vec<String>,
    instructor: Option<String>,
    title: Option<String>,
    only_allow_open: bool,
}

#[post("/search/<term>", format = "json", data = "<query>")]
async fn search_courses(term: String, query: Json<SearchQuery>) -> content::Json<String> {
    // kek I didn't realize how scuffed this was
    if let Some(wg_handler) = WEBREG_WRAPPERS.get(&term.as_str()) {
        let mut query_builder = SearchRequestBuilder::new();
        for q in &query.subjects {
            query_builder = query_builder.add_subject(q.as_str());
        }

        for q in &query.courses {
            query_builder = query_builder.add_course(q.as_str());
        }

        for q in &query.departments {
            query_builder = query_builder.add_department(q.as_str());
        }

        query_builder = match query.instructor {
            Some(ref r) => query_builder.set_instructor(r),
            None => query_builder,
        };

        query_builder = match query.title {
            Some(ref r) => query_builder.set_title(r),
            None => query_builder,
        };

        if query.only_allow_open {
            query_builder = query_builder.only_allow_open();
        }

        let wg_handler = wg_handler.wrapper.lock().await;
        let search_res = wg_handler
            .search_courses(SearchType::Advanced(&query_builder))
            .await;
        drop(wg_handler);
        return process_return(search_res);
    }

    content::Json(json!({"error": "Invalid term specified"}).to_string())
}

#[inline(always)]
fn process_return<T>(search_res: Output<T>) -> content::Json<String>
where
    T: Serialize,
{
    match search_res {
        Ok(x) => content::Json(serde_json::to_string(&x).unwrap_or_else(|_| "[]".to_string())),
        Err(e) => content::Json(json!({ "error": e }).to_string()),
    }
}

#[get("/course/<term>/<subj>/<num>")]
async fn get_course_info(term: String, subj: String, num: String) -> content::Json<String> {
    if let Some(wg_handler) = WEBREG_WRAPPERS.get(&term.as_str()) {
        let wg_handler = wg_handler.wrapper.lock().await;
        let res = wg_handler.get_course_info(&subj, &num).await;
        drop(wg_handler);
        process_return(res)
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
