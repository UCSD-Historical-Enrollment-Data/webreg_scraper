#![allow(dead_code)]
mod export;
mod schedule;
mod tracker;
mod util;

use crate::tracker::run_tracker;
use once_cell::sync::Lazy;
use rocket::response::content;
use rocket::{get, routes};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use webweg::webreg_wrapper::WebRegWrapper;

const TERM: &str = "SP22";
const VERSION: &str = env!("CARGO_PKG_VERSION");

static WEBREG_WRAPPER: Lazy<Arc<Mutex<WebRegWrapper>>> = Lazy::new(|| {
    let cookie = get_cookies();
    let cookie = cookie.trim();
    Arc::new(Mutex::new(WebRegWrapper::new(cookie.to_string(), TERM)))
});

// When I feel like everything's good enough, I'll probably make this into
// a better interface for general users.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("WebRegWrapper Version {}\n", VERSION);
    if !WEBREG_WRAPPER.lock().await.is_valid().await {
        eprintln!("Failed to login.");
        return Ok(());
    }

    println!(
        "Logged in successfully. Account name: {}",
        WEBREG_WRAPPER.lock().await.get_account_name().await
    );

    let clone = WEBREG_WRAPPER.clone();
    let jh = tokio::spawn(async move {
        run_tracker(clone, Some("http://localhost:3000/cookie")).await;
    });

    rocket::build()
        .mount("/", routes![get_course_info])
        .launch()
        .await
        .unwrap();

    // If, for some reason rocket dies, at least this will keep running
    jh.await.unwrap();
    Ok(())
}

#[get("/course/<subj>/<num>")]
async fn get_course_info(subj: String, num: String) -> content::Json<String> {
    let w = WEBREG_WRAPPER.lock().await;
    let res = w.get_course_info(&subj, &num).await;
    drop(w);
    match res {
        Ok(o) => content::Json(serde_json::to_string(&o).unwrap_or_else(|_| "[]".to_string())),
        Err(e) => {
            let mut s = String::from("{ \"error\": ");
            s.push_str(&format!("\"{}\" ", e));
            s.push_str("}");
            content::Json(s)
        },
    }
}

fn get_cookies() -> String {
    use std::fs;
    use std::path::Path;
    let file = Path::new("cookie.txt");
    if !file.exists() {
        eprintln!("'cookie.txt' file does not exist. Try again.");
        return "".to_string();
    }

    fs::read_to_string(file).unwrap_or_else(|_| "".to_string())
}
