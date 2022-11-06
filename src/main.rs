#![allow(dead_code)]
mod export;
mod git;
mod schedule;
mod tracker;
mod util;

use crate::tracker::run_tracker;

use once_cell::sync::Lazy;
use reqwest::Client;
use rocket::response::content;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{get, post, routes};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::wrapper::{
    CourseLevelFilter, SearchRequestBuilder, SearchType, WebRegWrapper, WrapperError,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The interval to start each instance. In other words, the number of seconds between starting
/// two scrapers.
pub const STARTUP_COOLDOWN: f64 = 1.5;

pub struct TermSetting<'a> {
    /// The term, to be recognized by WebReg.
    term: &'a str,

    /// The term alias, to be used for the file name. If `None`, defaults
    /// to `term`.
    alias: Option<&'a str>,

    /// The recovery URL, i.e., the URL the wrapper should
    /// make a request to so it can get new cookies to login
    /// with.
    port: Option<usize>,

    /// The cooldown, if any, between requests. If none is
    /// specified, then this will use the default cooldown.
    cooldown: f64,

    /// The courses to search for this term.
    search_query: Vec<SearchRequestBuilder<'static>>,

    /// Whether this term needs to be applied manually to the wrapper
    /// before use.
    apply_term: bool,
}

/// All terms and their associated "recovery URLs"
pub static TERMS: Lazy<Vec<TermSetting<'static>>> = Lazy::new(|| {
    Vec::from([
        TermSetting {
            term: "WI23",
            alias: None,
            port: Some(3001),
            apply_term: false,

            #[cfg(debug_assertions)]
            cooldown: 3.0,
            #[cfg(not(debug_assertions))]
            cooldown: 0.40,

            #[cfg(debug_assertions)]
            search_query: vec![SearchRequestBuilder::new()
                .filter_courses_by(CourseLevelFilter::LowerDivision)
                .filter_courses_by(CourseLevelFilter::UpperDivision)
                .add_department("MATH")
                .add_department("CSE")
                .add_department("COGS")],

            #[cfg(not(debug_assertions))]
            search_query: vec![
                // For fall, we want *all* lower- and upper-division courses
                SearchRequestBuilder::new()
                    .filter_courses_by(CourseLevelFilter::LowerDivision)
                    .filter_courses_by(CourseLevelFilter::UpperDivision),
                // But only graduate math/cse/ece/cogs courses
                SearchRequestBuilder::new()
                    .filter_courses_by(CourseLevelFilter::Graduate)
                    .add_department("MATH")
                    .add_department("CSE")
                    .add_department("ECE")
                    .add_department("COGS"),
            ],
        },
    ])
});

pub struct WebRegHandler<'a> {
    /// Wrapper for the scraper.
    scraper_wrapper: Mutex<WebRegWrapper>,

    /// Wrapper for general requests made inbound by, say, a Discord bot.
    general_wrapper: Mutex<WebRegWrapper>,

    /// The term settings.
    term_setting: &'a TermSetting<'a>,
}

// Init all wrappers here.
static WEBREG_WRAPPERS: Lazy<HashMap<&str, WebRegHandler>> = Lazy::new(|| {
    let mut map: HashMap<&str, WebRegHandler> = HashMap::new();

    for term_setting in TERMS.iter() {
        let cookie = get_file_content(&format!("cookie_{}.txt", term_setting.term));
        let cookie = cookie.trim();
        map.insert(
            term_setting.term,
            WebRegHandler {
                scraper_wrapper: Mutex::new(WebRegWrapper::new(
                    Client::new(),
                    cookie.to_string(),
                    term_setting.term,
                )),
                general_wrapper: Mutex::new(WebRegWrapper::new(
                    Client::new(),
                    cookie.to_string(),
                    term_setting.term,
                )),
                term_setting,
            },
        );
    }

    map
});

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

// The stop flag is our way of communicating to each wrapper that we're stopping
// the process completely. We use a stop flag so we can flush all non-empty buffers
// before quitting the program.
static STOP_FLAG: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// The num_stopped variable here tells us exactly how many wrappers have actually
// stopped. We want to ensure all wrappers are done working before we quit the
// program.
static NUM_STOPPED: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("WebRegWrapper Version {}", VERSION);

    for (_, wg_handler) in WEBREG_WRAPPERS.iter() {
        tokio::spawn(async move {
            // wg_handler has a static lifetime, so we can do this just fine.
            run_tracker(wg_handler).await;
        });

        tokio::time::sleep(Duration::from_secs_f64(STARTUP_COOLDOWN)).await;
    }

    let _ = rocket::build()
        .mount(
            "/",
            routes![get_course_info, search_courses, get_prereqs, get_stat],
        )
        .launch()
        .await
        .unwrap();

    // Rocket should only die if CTRL-C is detected
    println!("CTRL-C Detected. Stopping...");
    STOP_FLAG.store(true, Ordering::SeqCst);

    while NUM_STOPPED.load(Ordering::SeqCst) < WEBREG_WRAPPERS.len() {
        // Keep looping basically. Because we're working with a
        // single-threaded environment, we do want to sleep from time
        // to time so the other "green threads" have the opportunity
        // to run.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

#[inline(always)]
fn process_return<T>(search_res: Result<T, WrapperError>) -> content::RawJson<String>
where
    T: Serialize,
{
    match search_res {
        Ok(x) => content::RawJson(serde_json::to_string(&x).unwrap_or_else(|_| "[]".to_string())),
        Err(e) => content::RawJson(json!({ "error": e.to_string() }).to_string()),
    }
}

enum PortType {
    Found(usize),
    InvalidTerm,
    NoPortDefined,
}

// We could delegate these endpoints to the actual login script itself and have it
// deal with these requests, but I want to make everything more uniform by providing
// all endpoints here. Plus, all ports and terms are defined here anyways which makes
// it easier for the end user to use these endpoints (it would be more difficult if
// these endpoints had to be accessed from the login script itself).
#[get("/stat/<stat_type>/<term>")]
async fn get_stat(stat_type: String, term: String) -> content::RawJson<String> {
    let port_to_use = {
        if let Some(wg_handler) = WEBREG_WRAPPERS.get(&term.as_str()) {
            if let Some(port) = wg_handler.term_setting.port {
                PortType::Found(port)
            } else {
                PortType::NoPortDefined
            }
        } else {
            PortType::InvalidTerm
        }
    };

    match port_to_use {
        PortType::Found(port) => match stat_type.as_str() {
            "start" | "history" => {
                match CLIENT
                    .get(format!("http://localhost:{}/{}", port, stat_type))
                    .send()
                    .await
                {
                    Ok(o) => {
                        let default_val = match stat_type.as_str() {
                            "start" => "[]",
                            "history" => "0",
                            // Should never hit by earlier condition
                            _ => "{}",
                        };

                        let data = o.text().await.unwrap_or_else(|_| default_val.to_string());
                        content::RawJson(data)
                    }
                    Err(e) => content::RawJson(json!({ "error": format!("{}", e) }).to_string()),
                }
            }
            _ => content::RawJson(
                json!({ "error": format!("Invalid stat type: {}", stat_type) }).to_string(),
            ),
        },
        PortType::InvalidTerm => content::RawJson(
            json!({
                "error": "Invalid term specified."
            })
            .to_string(),
        ),
        PortType::NoPortDefined => content::RawJson(
            json!({
                "error": "Term exists but no port defined."
            })
            .to_string(),
        ),
    }
}

#[get("/course/<term>/<subj>/<num>")]
async fn get_course_info(term: String, subj: String, num: String) -> content::RawJson<String> {
    if let Some(wg_handler) = WEBREG_WRAPPERS.get(&term.as_str()) {
        let wg_handler = wg_handler.general_wrapper.lock().await;
        let res = wg_handler.get_course_info(&subj, &num).await;
        drop(wg_handler);
        process_return(res)
    } else {
        content::RawJson(
            json!({
                "error": "Invalid term specified."
            })
            .to_string(),
        )
    }
}

#[get("/prereqs/<term>/<subj>/<num>")]
async fn get_prereqs(term: String, subj: String, num: String) -> content::RawJson<String> {
    if let Some(wg_handler) = WEBREG_WRAPPERS.get(&term.as_str()) {
        let wg_handler = wg_handler.general_wrapper.lock().await;
        let res = wg_handler.get_prereqs(&subj, &num).await;
        drop(wg_handler);
        process_return(res)
    } else {
        content::RawJson(
            json!({
                "error": "Invalid term specified."
            })
            .to_string(),
        )
    }
}

fn get_file_content(file_name: &str) -> String {
    let file = Path::new(file_name);
    if !file.exists() {
        eprintln!("'{}' file does not exist. Try again.", file_name);
        return "".to_string();
    }

    fs::read_to_string(file).unwrap_or_else(|_| "".to_string())
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct SearchQuery {
    subjects: Vec<String>,
    courses: Vec<String>,
    departments: Vec<String>,
    instructor: Option<String>,
    title: Option<String>,
    only_allow_open: bool,
    show_lower_div: bool,
    show_upper_div: bool,
    show_grad_div: bool,
}

#[post("/search/<term>", format = "json", data = "<query>")]
async fn search_courses(term: String, query: Json<SearchQuery>) -> content::RawJson<String> {
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

        if query.show_grad_div {
            query_builder = query_builder.filter_courses_by(CourseLevelFilter::Graduate);
        }

        if query.show_upper_div {
            query_builder = query_builder.filter_courses_by(CourseLevelFilter::UpperDivision);
        }

        if query.show_lower_div {
            query_builder = query_builder.filter_courses_by(CourseLevelFilter::LowerDivision);
        }

        let wg_handler = wg_handler.general_wrapper.lock().await;
        let search_res = wg_handler
            .search_courses(SearchType::Advanced(&query_builder))
            .await;
        drop(wg_handler);
        return process_return(search_res);
    }

    content::RawJson(json!({"error": "Invalid term specified"}).to_string())
}
