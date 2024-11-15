use crate::scraper::tracker::run_tracker;
use crate::server::create_router;
use crate::types::{ConfigScraper, WrapperState};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::log::{error, info, warn};

mod scraper;
mod server;
mod types;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    tokio::spawn({
        let cloned_state = state.clone();
        async move {
            run_tracker(cloned_state, is_verbose).await;
        }
    });

    let addr = SocketAddr::from_str(
        format!(
            "{}:{}",
            state.api_base_endpoint.address.as_str(),
            state.api_base_endpoint.port
        )
        .as_str(),
    );

    info!(
        "Server started on address {}:{}",
        state.api_base_endpoint.address.as_str(),
        state.api_base_endpoint.port
    );

    let listener = tokio::net::TcpListener::bind(&addr.unwrap()).await.unwrap();
    axum::serve(listener, create_router(state.clone()).into_make_service())
        .with_graceful_shutdown(shutdown_signal(state))
        .await
        .unwrap();
    ExitCode::SUCCESS
}

/// Handles shutting down the server.
///
/// # Parameters
/// - `state`: The wrapper state, which is a reference to all valid scrapers and other relevant
///   information.
async fn shutdown_signal(state: Arc<WrapperState>) {
    tokio::signal::ctrl_c()
        .await
        .expect("Expected shutdown signal handler.");

    // Intercept ctrl_c event
    warn!("Invoked ctrl+c event, stopping the scraper and server.");
    state.set_stop_flag(true);
    while state.is_running() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
