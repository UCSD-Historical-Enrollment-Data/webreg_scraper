use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use webweg::reqwest::Client;
use webweg::wrapper::{CourseLevelFilter, SearchRequestBuilder, WebRegWrapper};

#[derive(Clone)]
pub struct WrapperState {
    pub all_wrappers: WrapperMap,
    pub stop_flag: Arc<AtomicBool>,
    pub stop_ct: Arc<AtomicUsize>,
}

pub type WrapperMap = HashMap<String, Arc<TermInfo>>;

/// A structure that holds information relating to the scraper and, more importantly, the
/// scraper instances themselves.
pub struct TermInfo {
    pub term: String,
    pub alias: Option<String>,
    pub recovery: Option<AddressPortInfo>,
    pub cooldown: f64,
    pub search_query: Vec<SearchRequestBuilder>,
    pub apply_term: bool,
    pub scraper_wrapper: Mutex<WebRegWrapper>,
    pub general_wrapper: Mutex<WebRegWrapper>,
}

impl From<&ConfigTermDatum> for TermInfo {
    fn from(value: &ConfigTermDatum) -> Self {
        let mut info = TermInfo {
            term: value.term.to_owned(),
            alias: value.alias.to_owned(),
            recovery: value.recovery_info.to_owned(),
            cooldown: value.cooldown,
            search_query: vec![],
            apply_term: value.apply_before_use,
            scraper_wrapper: Mutex::new(WebRegWrapper::new(Client::new(), "", value.term.as_str())),
            general_wrapper: Mutex::new(WebRegWrapper::new(Client::new(), "", value.term.as_str())),
        };

        if cfg!(feature = "scraper") {
            // If we're working with the feature, then we must transfer all queries from the
            // configuration file to here.
            for query in &value.search_query {
                let mut parsed_query = SearchRequestBuilder::new();
                for level in &query.levels {
                    parsed_query = match level.as_str() {
                        "g" => parsed_query.filter_courses_by(CourseLevelFilter::Graduate),
                        "u" => parsed_query.filter_courses_by(CourseLevelFilter::UpperDivision),
                        "l" => parsed_query.filter_courses_by(CourseLevelFilter::LowerDivision),
                        _ => continue,
                    };
                }

                for dept in &query.departments {
                    parsed_query = parsed_query.add_department(dept);
                }

                info.search_query.push(parsed_query);
            }
        } else {
            // Otherwise, we're not working with the scraper. This means we're working with
            // the API and thus only need to add a dummy course.
            info.search_query
                .push(SearchRequestBuilder::new().add_course("CSE 100"));
        }

        info
    }
}

/// A structure that represents a configuration file specifically for the scraper. An example
/// configuration file may look like the following:
/// ```json
/// {
///     "configName": "DEBUG",
///     "address": "127.0.0.1",
///     "port": 8080,
///     "wrapperData": [
///         {
///             "term": "WI23",
///             "recoveryInfo": {
///                 "address": "127.0.0.1",
///                 "port": 3000
///             },
///             "cooldown": 1.25,
///             "searchQuery": [
///                 {
///                     "levels": ["g"],
///                     "departments": ["MATH", "CSE", "COGS", "ECE"]
///                 },
///                 {
///                     "levels": ["l", "u", "g"],
///                     "departments": []
///                 }
///             ],
///             "applyBeforeUse": false
///         },
///         {
///             "term": "FA22",
///             "alias": "FA22NS",
///             "cooldown": 1.25,
///             "searchQuery": [
///                 {
///                     "levels": ["l", "u", "g"],
///                     "departments": ["MATH", "CSE", "COGS"]
///                 }
///             ],
///             "applyBeforeUse": false
///         }
///     ]
/// }
/// ```
#[derive(Serialize, Deserialize)]
pub struct ConfigScraper {
    /// The name of the configuration. This is used solely for making it easier to
    /// identify.
    #[serde(rename = "configName")]
    pub config_name: String,
    /// The address for which the endpoints specified in this application is made
    /// available for other applications to use.
    #[serde(rename = "apiInfo")]
    pub api_info: AddressPortInfo,
    /// Information about what terms the scraper will be gathering data for.
    #[serde(rename = "wrapperData")]
    pub terms: Vec<ConfigTermDatum>,
    /// Whether the logging should be verbose or not.
    pub verbose: bool,
}

/// A structure that represents a specific term that the scraper should consider.
#[derive(Serialize, Deserialize)]
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
    /// The recovery address/port information. When the scraper is unable to get data
    /// for this particular term, it will attempt to request new session cookies for this
    /// term so it can continue to get data.
    ///
    /// If this is not specified, then the program will exit upon failing to get any data.
    #[serde(rename = "recoveryInfo")]
    pub recovery_info: AddressPortInfo,
    /// The delay between each individual request for a course, in seconds.
    pub cooldown: f64,
    /// The courses that the scraper should be gathering data for.
    #[serde(rename = "searchQuery")]
    pub search_query: Vec<ConfigSearchQuery>,
    /// Whether to force the scraper to apply the term to it before scraping data. This is
    /// useful when, for example, the term is valid (i.e., it is in the WebReg system) but
    /// cannot be accessed through normal means.
    #[serde(rename = "applyBeforeUse")]
    pub apply_before_use: bool,
    /// The term alias. This is used in place of the `term` for the file name. If no such
    /// alias is specified, this defaults to the `term`.
    pub alias: Option<String>,
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

/// A structure that represents an address and port.
#[derive(Serialize, Deserialize, Clone)]
pub struct AddressPortInfo {
    /// The address.
    pub address: String,
    /// The port.
    pub port: i64,
}
