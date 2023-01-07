#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use webweg::reqwest::Client;

#[cfg(feature = "api")]
use {
    crate::api::status_api::{api_get_login_script_stats, api_get_term_status},
    crate::api::webreg_api::{api_get_course_info, api_get_prereqs, api_get_search_courses},
    axum::routing::get,
    axum::Router,
};

use crate::tracker::run_tracker;
use crate::types::{ConfigScraper, WrapperMap, WrapperState};

mod api;
mod tracker;
mod types;
mod util;

#[cfg(not(any(feature = "scraper", feature = "api")))]
compile_error!("A feature ('scraper' and/or 'api') must be specified!");

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The interval to start each instance. In other words, the number of seconds between starting
/// two scrapers.
pub const STARTUP_COOLDOWN: f64 = 1.5;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    println!("WebRegScraper/API Version {}", VERSION);
    // First, get the configuration file.
    let config_path = match std::env::args().skip(1).last() {
        Some(s) => s,
        None => {
            println!("[!] Please provide the path to a configuration file for the scraper.");
            return ExitCode::FAILURE;
        }
    };

    let config_path = Path::new(config_path.as_str());
    if !config_path.exists() {
        println!("[!] Invalid path. Please provide the path to a configuration file.");
        return ExitCode::FAILURE;
    }

    let config_info = match serde_json::from_str::<ConfigScraper>(
        fs::read_to_string(config_path)
            .expect("Unable to read file.")
            .as_str(),
    ) {
        Ok(config) => config,
        Err(err) => {
            println!(
                "[!] Bad config file. Please fix it and then try again.\n{}",
                err
            );
            return ExitCode::FAILURE;
        }
    };

    println!("Loaded: {}", config_info.config_name);
    #[cfg(feature = "scraper")]
    println!("\twith feature: scraper");
    #[cfg(feature = "api")]
    println!("\twith feature: api");

    let mut all_terms: WrapperMap = HashMap::new();
    for info in &config_info.terms {
        all_terms.insert(info.term.to_owned(), Arc::new(info.into()));
    }

    // These two variables will be used to determine whether the scraper needs to stop.
    let main_num_stopped = Arc::new(AtomicUsize::new(0));
    let main_stop_flag = Arc::new(AtomicBool::new(false));

    let state = WrapperState {
        all_wrappers: all_terms,
        stop_flag: main_stop_flag.clone(),
        stop_ct: main_num_stopped.clone(),
        client: Arc::new(Client::new()),
    };

    for (_, term_info) in state.all_wrappers.iter() {
        let this_term_info = term_info.clone();
        let this_stop_flag = state.stop_flag.clone();
        let this_stop_ct = main_num_stopped.clone();
        tokio::spawn(async move {
            run_tracker(this_term_info, this_stop_flag, this_stop_ct).await;
        });

        tokio::time::sleep(Duration::from_secs_f64(STARTUP_COOLDOWN)).await;
    }

    #[cfg(feature = "api")]
    {
        let app = Router::new()
            .route("/webreg/get_course_info/:term", get(api_get_course_info))
            .route("/webreg/get_prereqs/:term", get(api_get_prereqs))
            .route("/webreg/search_courses/:term", get(api_get_search_courses))
            .route("/scraper/term_status/:term", get(api_get_term_status))
            .route(
                "/scraper/login_script/:term/:stat_type",
                get(api_get_login_script_stats),
            )
            .with_state(state);

        // with_graceful_shutdown
        // https://github.com/joelparkerhenderson/demo-rust-axum/blob/main/src/main.rs
        // line 107
        axum::Server::bind(
            &format!(
                "{}:{}",
                config_info.api_info.address, config_info.api_info.port
            )
            .parse()
            .unwrap(),
        )
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal(
            main_num_stopped.clone(),
            main_stop_flag.clone(),
            config_info.terms.len(),
        ))
        .await
        .unwrap();
    }

    #[cfg(not(feature = "api"))]
    {
        shutdown_signal(
            main_num_stopped.clone(),
            main_stop_flag.clone(),
            config_info.terms.len(),
        )
        .await;
    }

    ExitCode::SUCCESS
}

/// Handles shutting down the server.
///
/// # Parameters
/// - `num_stopped`: The number of scrapers that have stopped.
/// - `stop_flag`: The flag indicating whether the scrapers should stop.
/// - `num_total`: The total number of terms.
async fn shutdown_signal(
    num_stopped: Arc<AtomicUsize>,
    stop_flag: Arc<AtomicBool>,
    num_total: usize,
) {
    tokio::signal::ctrl_c()
        .await
        .expect("Expected shutdown signal handler.");

    stop_flag.store(true, Ordering::SeqCst);
    while num_stopped.load(Ordering::SeqCst) < num_total {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
