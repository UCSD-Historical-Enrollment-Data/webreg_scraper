use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use webweg::wrapper::input_types::{CourseLevelFilter, SearchRequestBuilder};
use webweg::wrapper::WebRegWrapper;

const MAX_RECENT_REQUESTS: usize = 2000;

/// A structure that represents the current state of all wrappers.
pub struct WrapperState {
    /// A map containing all active scrapers, grouped by term.
    pub all_terms: WrapperMap,
    /// The stop flag; i.e., the flag that indicates whether the scraper should be stopped.
    pub stop_flag: AtomicBool,
    /// Whether the scrapers are running at this moment.
    pub is_running: AtomicBool,
    /// The client that can be used to make requests.
    pub client: Client,
    /// The wrapper that can be used to make requests to WebReg.
    pub wrapper: WebRegWrapper,
    /// A wrapper to be used to serve requests that involve other cookies.
    pub c_wrapper: WebRegWrapper,
    /// The address for which the endpoints specified in this application is made
    /// available for other applications to use.
    pub api_base_endpoint: AddressPortInfo,
    /// The cookie server.
    pub cookie_server: AddressPortInfo,
    /// The authentication manager, to be used by the server.
    #[cfg(feature = "auth")]
    pub auth_manager: basicauth::AuthManager,
}

impl WrapperState {
    /// Creates a new `WrapperState` using the given configuration scraper.
    ///
    /// # Parameter
    /// - `config`: The configuration data.
    ///
    /// # Returns
    /// The wrapper state.
    pub fn new(config: ConfigScraper) -> Self {
        let term_info: WrapperMap = config
            .wrapper_data
            .into_iter()
            .map(|data| TermInfo {
                term: data.term,
                cooldown: data.cooldown,
                search_query: data
                    .search_query
                    .into_iter()
                    .map(|query| {
                        let mut parsed = SearchRequestBuilder::new();
                        for level in query.levels {
                            parsed = match level.as_str() {
                                "g" => parsed.filter_courses_by(CourseLevelFilter::Graduate),
                                "u" => parsed.filter_courses_by(CourseLevelFilter::UpperDivision),
                                "l" => parsed.filter_courses_by(CourseLevelFilter::LowerDivision),
                                _ => continue,
                            };
                        }

                        for dept in query.departments {
                            parsed = parsed.add_department(dept);
                        }
                        parsed
                    })
                    .collect(),
                tracker: StatTracker {
                    recent_requests: Default::default(),
                    num_requests: Default::default(),
                    total_time_spent: Default::default(),
                },
                should_save: data.save_data_to_file,
            })
            .map(|data| (data.term.to_owned(), Arc::new(data)))
            .collect();

        Self {
            all_terms: term_info,
            stop_flag: AtomicBool::from(false),
            is_running: AtomicBool::from(false),
            client: Default::default(),
            wrapper: WebRegWrapper::builder()
                .with_cookies("To be loaded later")
                .try_build_wrapper()
                .unwrap(),
            c_wrapper: WebRegWrapper::builder()
                .with_cookies("To be determined by the user's cookies.")
                .should_close_after_request(true)
                .try_build_wrapper()
                .unwrap(),
            api_base_endpoint: config.api_base_endpoint,
            cookie_server: config.cookie_server,
            #[cfg(feature = "auth")]
            auth_manager: basicauth::AuthManager::new(),
        }
    }

    /// Gets the current status of the stop flag.
    ///
    /// # Returns
    /// `true` if the stop flag is set to TRUE, indicating that the scraper and all
    /// associated functions should stop.
    pub fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::SeqCst)
    }

    /// Sets the stop flag to the specified value.
    ///
    /// # Parameters
    /// - `stop_status`: The stop status to set.
    pub fn set_stop_flag(&self, stop_status: bool) {
        self.stop_flag.store(stop_status, Ordering::SeqCst);
    }

    /// Indicates whether the scraper for _all_ terms is running.
    ///
    /// # Returns
    /// `true` if the scraper is running, and `false` otherwise.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

pub type WrapperMap = HashMap<String, Arc<TermInfo>>;

/// A structure that holds basic stats about the tracker's requests.
#[derive(Default)]
pub struct StatTracker {
    /// The amount of time it took for the 100 most requests to finish processing.
    pub recent_requests: Mutex<VecDeque<usize>>,
    /// The number of requests that have been made thus far.
    pub num_requests: AtomicUsize,
    /// The total amount of time spent making those requests, in milliseconds.
    pub total_time_spent: AtomicUsize,
}

impl StatTracker {
    /// Adds a stat to the `StatTracker` instance.
    ///
    /// # Parameters
    /// - `time_of_req`: The time it took to make a request.
    pub fn add_stat(&self, time_of_req: usize) {
        self.num_requests.fetch_add(1, Ordering::SeqCst);
        self.total_time_spent
            .fetch_add(time_of_req, Ordering::SeqCst);
        let mut recent_requests = self.recent_requests.lock().unwrap();
        while recent_requests.len() >= MAX_RECENT_REQUESTS {
            recent_requests.pop_front();
        }

        recent_requests.push_back(time_of_req);
    }
}

/// A structure that holds information relating to the scraper and, more importantly, the
/// scraper instances themselves.
pub struct TermInfo {
    /// The term associated with this scraper.
    pub term: String,
    /// The cooldown, in seconds, between requests.
    pub cooldown: f64,
    /// The courses to search for.
    pub search_query: Vec<SearchRequestBuilder>,
    /// Tracker stats. This field contains information on the performance of the scraper.
    pub tracker: StatTracker,
    /// Whether we should save data scraped for this term to a file.
    pub should_save: bool,
}

/// A structure that represents a configuration file specifically for the scraper. See the
/// `config.example.json` file and the README for documentation.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigScraper {
    /// The name of the configuration. This is used solely for making it easier to
    /// identify.
    pub config_name: String,
    /// The address for which the endpoints specified in this application is made
    /// available for other applications to use.
    pub api_base_endpoint: AddressPortInfo,
    /// The recovery address/port information. When the scraper is unable to get data
    /// for this particular term, it will attempt to request new session cookies for this
    /// term so it can continue to get data.
    pub cookie_server: AddressPortInfo,
    /// Information about what terms the scraper will be gathering data for.
    pub wrapper_data: Vec<ConfigTermDatum>,
    /// Whether the logging should be verbose or not.
    pub verbose: bool,
}

/// A structure that represents an address and port.
#[derive(Serialize, Deserialize, Clone)]
pub struct AddressPortInfo {
    /// The address.
    pub address: String,
    /// The port.
    pub port: i64,
}

/// A structure that represents a specific term that the scraper should consider.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigTermDatum {
    /// The term, represented by four characters. The first two characters must be
    /// one of the following:
    /// - `FA` for Fall term
    /// - `WI` for Winter term
    /// - `SP` for Spring term
    /// - `S1` for Summer 1 term
    /// - `S2` for Summer 2 term
    ///
    /// The last two characters must represent the year associated with that term.
    /// For example, `FA22` represents the Fall 2022 term, and `S120` represents the
    /// Summer 1 2020 term.
    pub term: String,
    /// The delay between each individual request for a course, in seconds.
    pub cooldown: f64,
    /// The courses that the scraper should be gathering data for.
    pub search_query: Vec<ConfigSearchQuery>,
    /// Whether we should be saving data scraped for this term to a file.
    pub save_data_to_file: bool,
}

/// A structure that represents a search query for a term for the scraper.
#[derive(Serialize, Deserialize)]
pub struct ConfigSearchQuery {
    /// The course levels to consider. Three levels are currently recognized:
    /// - `g`: graduate courses
    /// - `u`: upper-division courses
    /// - `l`: lower-division courses
    pub levels: Vec<String>,
    /// The departments to consider. Use the department's code here. If no department is
    /// specified, then all courses will be fetched.
    pub departments: Vec<String>,
}
