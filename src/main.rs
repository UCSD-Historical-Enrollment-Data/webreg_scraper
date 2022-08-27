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
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::webreg_wrapper::{
    CourseLevelFilter, Output, SearchRequestBuilder, SearchType, WebRegWrapper,
};

cfg_feature_git! {
    use crate::git::GitManager;
    use crate::util::get_pretty_time;
    use chrono::Local;
    use std::thread;
}

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
    recovery_url: Option<&'a str>,

    /// The cooldown, if any, between requests. If none is
    /// specified, then this will use the default cooldown.
    cooldown: f64,

    /// The courses to search for this term.
    search_query: Vec<SearchRequestBuilder<'static>>,
}

/// All terms and their associated "recovery URLs"
pub static TERMS: Lazy<Vec<TermSetting<'static>>> = Lazy::new(|| {
    vec![TermSetting {
        term: "FA22",
        alias: Some("FA22A"),
        recovery_url: Some("http://localhost:3000/cookie"),

        #[cfg(debug_assertions)]
        cooldown: 3.0,
        #[cfg(not(debug_assertions))]
        cooldown: 0.42,

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
    }]
});

pub struct WebRegHandler<'a> {
    /// Wrapper for the scraper.
    scraper_wrapper: Mutex<WebRegWrapper<'a>>,

    /// Wrapper for general requests made inbound by, say, a Discord bot.
    general_wrapper: Mutex<WebRegWrapper<'a>>,

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("WebRegWrapper Version {}", VERSION);

    #[cfg(feature = "git_repeat")]
    println!("\tUsing Git Auto-Commit.");

    // Location to store the cleaned CSV files. For example, if the base folder
    // that we want to save these files to is `../UCSDEnrollmentData` (with child
    // directories, say, `UCSDEnrollmentData/FA22`, `UCSDEnrollmentData/SP22`,
    // etc.), then we would just have `../UCSDEnrollmentData`.
    #[cfg(feature = "git_repeat")]
    let clean_loc = {
        let t = get_file_content("clean.txt");
        if t.is_empty() {
            // exit process
            panic!();
        }

        t
    };

    #[cfg(feature = "git_repeat")]
    {
        let path = Path::new(&clean_loc);
        if !path.exists() {
            panic!("Path {} does not exist. Please create the directory and any associated child directories.", 
            path.display());
        }
        println!("Set path for clean data: {}", path.display());
    }

    for (_, wg_handler) in WEBREG_WRAPPERS.iter() {
        #[cfg(feature = "git_repeat")]
        let loc = clean_loc.clone();
        tokio::spawn(async move {
            // wg_handler has a static lifetime, so we can do this just fine.
            #[cfg(feature = "git_repeat")]
            run_tracker(wg_handler, loc).await;
            #[cfg(not(feature = "git_repeat"))]
            run_tracker(wg_handler).await;
        });

        tokio::time::sleep(Duration::from_secs_f64(STARTUP_COOLDOWN)).await;
    }

    // Spawn a thread to handle git pull/push, since Command::new()...status()
    // is a blocking call which can potentially cause problems if we're pulling
    // or pushing a *lot* of files.
    #[cfg(feature = "git_repeat")]
    thread::spawn(move || {
        let loc = clean_loc;
        let git = GitManager::new(Path::new(&loc));
        loop {
            thread::sleep(Duration::from_secs(5 * 60));
            println!("[GIT] [{}] Running Git service.", get_pretty_time());
            git.pull_files();
            thread::sleep(Duration::from_secs(2));
            git.add_all_files();
            thread::sleep(Duration::from_secs(2));

            let commit_msg = Local::now().format("%B %d, %Y at %I:%M:%S %p").to_string();
            git.commit_files(&format!("{} - Update (Automated)", commit_msg));
            thread::sleep(Duration::from_secs(2));

            git.push_files();
            println!("[GIT] [{}] Git service finished.", get_pretty_time());
            thread::sleep(Duration::from_secs(2));
        }
    });

    let _ = rocket::build()
        .mount("/", routes![get_course_info, search_courses, get_prereqs])
        .launch()
        .await
        .unwrap();

    Ok(())
}

#[inline(always)]
fn process_return<T>(search_res: Output<T>) -> content::RawJson<String>
where
    T: Serialize,
{
    match search_res {
        Ok(x) => content::RawJson(serde_json::to_string(&x).unwrap_or_else(|_| "[]".to_string())),
        Err(e) => content::RawJson(json!({ "error": e }).to_string()),
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

        let wg_handler = wg_handler.general_wrapper.lock().await;
        let search_res = wg_handler
            .search_courses(SearchType::Advanced(&query_builder))
            .await;
        drop(wg_handler);
        return process_return(search_res);
    }

    content::RawJson(json!({"error": "Invalid term specified"}).to_string())
}
