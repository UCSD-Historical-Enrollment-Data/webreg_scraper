use crate::tracker::run_tracker;
use crate::types::{ConfigScraper, WrapperState};
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tracing::log::{error, info};

mod tracker;
mod types;
mod util;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The interval to start each instance. In other words, the number of seconds between starting
/// two scrapers.
pub const STARTUP_COOLDOWN: f64 = 1.5;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    info!("Started webreg_scraper, version {VERSION}");
    // First, get the configuration file.
    let config_path = match std::env::args().skip(1).last() {
        Some(s) => s,
        None => {
            error!("Provide a path to the configuration JSON file as an argument.");
            return ExitCode::FAILURE;
        }
    };

    let config_path = Path::new(config_path.as_str());
    if !config_path.exists() {
        error!("Invalid path. Please provide the path to a configuration file.");
        return ExitCode::FAILURE;
    }

    let config_info = match serde_json::from_str::<ConfigScraper>(
        fs::read_to_string(config_path)
            .expect("Unable to read file.")
            .as_str(),
    ) {
        Ok(config) => config,
        Err(err) => {
            error!("Bad config file. Please fix it and then try again.\n{err}");
            return ExitCode::FAILURE;
        }
    };

    let is_verbose = config_info.verbose;
    info!("Loaded configuration file: {}", config_info.config_name);

    // Run the tracker for each term
    let state = Arc::new(WrapperState::new(config_info));
    for (_, term_info) in state.all_terms.iter() {
        let this_state = state.clone();
        let this_term_info = term_info.clone();
        tokio::spawn(async move {
            run_tracker(this_state, this_term_info, is_verbose).await;
        });

        tokio::time::sleep(Duration::from_secs_f64(STARTUP_COOLDOWN)).await;
    }

    shutdown_signal(state.clone()).await;
    ExitCode::SUCCESS
}

/// Handles shutting down the server.
///
/// # Parameters
/// - `state`: The wrapper state, which is a reference to all valid scrapers and other relevant
/// information.
async fn shutdown_signal(state: Arc<WrapperState>) {
    tokio::signal::ctrl_c()
        .await
        .expect("Expected shutdown signal handler.");

    // Intercept ctrl_c event
    info!("Invoked ctrl+c event, stopping the scraper and server.");
    state.stop_flag.store(true, Ordering::SeqCst);
    while state.is_running() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
