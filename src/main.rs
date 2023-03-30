#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use webweg::reqwest::Client;

#[cfg(feature = "api")]
use {
    crate::api::status_api::{
        api_get_login_script_stats, api_get_term_status, api_get_timing_stats,
    },
    crate::api::webreg_api::{api_get_course_info, api_get_prereqs, api_get_search_courses},
    axum::routing::get,
    axum::Router,
};
#[cfg(feature = "scraper")]
use {std::sync::atomic::Ordering, tracing::info};

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
    println!("WebRegScraper/API Version {VERSION}");
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
            println!("[!] Bad config file. Please fix it and then try again.\n{err}");
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
    let main_stop_flag = Arc::new(AtomicBool::new(false));

    let state = Arc::new(WrapperState {
        all_wrappers: all_terms,
        stop_flag: main_stop_flag.clone(),
        client: Arc::new(Client::new()),
    });

    for (_, term_info) in state.all_wrappers.iter() {
        let this_term_info = term_info.clone();
        let this_stop_flag = state.stop_flag.clone();
        tokio::spawn(async move {
            run_tracker(this_term_info, this_stop_flag, config_info.verbose).await;
        });

        tokio::time::sleep(Duration::from_secs_f64(STARTUP_COOLDOWN)).await;
    }

    #[cfg(feature = "api")]
    {
        let app = Router::new()
            .route("/webreg/course_info/:term", get(api_get_course_info))
            .route("/webreg/prereqs/:term", get(api_get_prereqs))
            .route("/webreg/search_courses/:term", get(api_get_search_courses))
            .route("/scraper/term_status/:term", get(api_get_term_status))
            .route(
                "/scraper/login_script/:term/:stat_type",
                get(api_get_login_script_stats),
            )
            .route("/scraper/timing_stats/:term", get(api_get_timing_stats))
            .with_state(state.clone());

        let server = axum::Server::bind(
            &format!(
                "{}:{}",
                config_info.api_info.address, config_info.api_info.port
            )
            .parse()
            .unwrap(),
        )
        .serve(app.into_make_service());

        // With the API feature enabled, we need to consider two cases.

        // Case 1:
        // If the scraper feature is ENABLED, then we need to make sure all scrapers
        // are stopped before we shut the process down. That way, any remaining data
        // in the buffers are written to the file.
        #[cfg(feature = "scraper")]
        server
            .with_graceful_shutdown(shutdown_signal(state.clone(), main_stop_flag.clone()))
            .await
            .unwrap();

        // Case 2:
        // Otherwise, we can just shut the server down without needing to wait for
        // anything.
        #[cfg(not(feature = "scraper"))]
        server
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Expected shutdown signal handler.");

                println!("Web server has been stopped.");
            })
            .await
            .unwrap();
    }

    // Otherwise, if we're not using the API feature, then we must have
    // the scraper feature.
    #[cfg(not(feature = "api"))]
    {
        shutdown_signal(state.clone(), main_stop_flag.clone()).await;
    }

    println!("Exiting.");
    ExitCode::SUCCESS
}

/// Handles shutting down the server.
///
/// # Parameters
/// - `state`: The wrapper state, which is a reference to all valid scrapers and other relevant
/// information.
/// - `stop_flag`: The flag indicating whether the scrapers should stop.
#[cfg(feature = "scraper")]
async fn shutdown_signal(state: Arc<WrapperState>, stop_flag: Arc<AtomicBool>) {
    tokio::signal::ctrl_c()
        .await
        .expect("Expected shutdown signal handler.");

    // Intercept ctrl_c event
    info!("Invoked ctrl+c event, attempting to stop scrapers.");
    stop_flag.store(true, Ordering::SeqCst);

    while state
        .all_wrappers
        .values()
        .filter(|x| !x.is_running.load(Ordering::SeqCst))
        .count()
        < state.all_wrappers.len()
    {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("\tScrapers stopped successfully.");
}
