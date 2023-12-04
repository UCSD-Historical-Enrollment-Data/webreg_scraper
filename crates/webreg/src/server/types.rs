use std::borrow::Cow;
use std::fmt::Debug;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use webweg::types::{SectionIdNotFoundContext, WrapperError};
use webweg::wrapper::input_types::{
    CourseLevelFilter, DayOfWeek, SearchRequestBuilder, SearchType,
};

#[derive(Deserialize, Debug)]
pub struct BodySectionId {
    #[serde(rename = "sectionId")]
    pub section_id: String,
}

#[derive(Deserialize, Debug)]
pub struct BodySectionScheduleNameId {
    #[serde(rename = "sectionId")]
    pub section_id: String,

    #[serde(rename = "scheduleName")]
    pub schedule_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct BodyScheduleNameChange {
    #[serde(rename = "oldName")]
    pub old_name: String,

    #[serde(rename = "newName")]
    pub new_name: String,
}

#[derive(Deserialize, Debug)]
pub struct BodyAddInfo {
    #[serde(rename = "sectionId")]
    pub section_id: String,
    #[serde(rename = "gradingOption")]
    pub grading_option: Option<String>,
    #[serde(rename = "unitCount")]
    pub unit_count: Option<i64>,
    #[serde(rename = "validate")]
    pub validate: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct BodyPlanAdd {
    #[serde(rename = "subjectCode")]
    pub subject_code: String,
    #[serde(rename = "courseCode")]
    pub course_code: String,
    #[serde(rename = "sectionId")]
    pub section_id: String,
    #[serde(rename = "sectionCode")]
    pub section_code: String,
    #[serde(rename = "gradingOption")]
    pub grading_option: Option<String>,
    #[serde(rename = "scheduleName")]
    pub schedule_name: Option<String>,
    #[serde(rename = "unitCount")]
    pub unit_count: i64,
    pub validate: Option<bool>,
}

/// A structure meant for a query string, intended to require the user to provide a name
/// for the schedule.
#[derive(Deserialize, Debug)]
pub struct ScheduleQueryStr {
    pub name: Option<String>,
}

/// A structure meant for a query string, intended to have the user provide a course to
/// search up in some way.
#[derive(Deserialize, Debug)]
pub struct CourseQueryStr {
    pub subject: String,
    pub number: String,
}

/// A structure meant for a query string, intended to have the user provide a "list" of
/// subject code (e.g., CSE)
#[derive(Deserialize, Debug)]
pub struct SubjListQueryStr {
    pub subjects: String,
}

/// A structure meant for a query string, intended to give users the ability to control
/// the type of response they wanted.
#[derive(Deserialize, Debug)]
pub struct RawQueryStr {
    pub raw: Option<bool>,
}

/// An enum that represents some sort of an error by the API.
pub enum ApiErrorType<'a> {
    /// Whether the error was from WebReg.
    WebReg(WrapperError),

    /// Whether the error is custom-made.
    General(StatusCode, Cow<'a, str>, Option<String>),
}

impl<'a> From<WrapperError> for ApiErrorType<'a> {
    fn from(value: WrapperError) -> Self {
        Self::WebReg(value)
    }
}

impl<'a, T> From<(StatusCode, T, Option<String>)> for ApiErrorType<'a>
where
    T: Into<Cow<'a, str>>,
{
    fn from((status, base, additional): (StatusCode, T, Option<String>)) -> Self {
        Self::General(status, base.into(), additional)
    }
}

impl<'a> IntoResponse for ApiErrorType<'a> {
    fn into_response(self) -> Response {
        let (status_code, base_error, additional_error) = match self {
            ApiErrorType::WebReg(err) => match err {
                WrapperError::RequestError(r) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "An internal request error occurred.".into(), Some(r.to_string()))
                }
                WrapperError::UrlParseError(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "An internal URL parsing error occurred.".into(), None)
                }
                WrapperError::InputError(i, e) => {
                    (StatusCode::BAD_REQUEST, "A bad argument was passed in.".into(), Some(format!("input={i}, bad arg value={e}")))
                }
                WrapperError::SerdeError(s) => {
                    (StatusCode::IM_A_TEAPOT, "An error occurred when trying to convert a string to a JSON object. It's possible your session is not valid.".into(), Some(s.to_string()))
                }
                WrapperError::BadStatusCode(b, c) => {
                    (StatusCode::from_u16(b).unwrap(), "A non-OK status code was hit.".into(), c)
                }
                WrapperError::WebRegError(w) => {
                    (StatusCode::BAD_REQUEST, "WebReg returned an error regarding your request.".into(), Some(w))
                }
                WrapperError::SectionIdNotFound(s, c) => {
                    let base = match c {
                        SectionIdNotFoundContext::Schedule => {
                            "The section ID you specified wasn't found in your schedule.".into()
                        }
                        SectionIdNotFoundContext::Catalog => {
                            "The section ID you specified doesn't appear to be offered in the specified term.".into()
                        }
                    };

                    (StatusCode::NOT_FOUND, base, Some(s))
                }
                WrapperError::WrapperParsingError(p) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "An error occurred when trying to convert the response JSON into an object.".into(), Some(p))
                }
                WrapperError::SessionNotValid => {
                    (StatusCode::UNAUTHORIZED, "Your session isn't valid. Try a different set of WebReg cookies.".into(), None)
                }
                WrapperError::BadTimeError => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "An error occurred when trying to parse a time unit.".into(), None)
                }
            }
            ApiErrorType::General(code, err, additional_info) => {
                (code, err, additional_info)
            }
        };

        let json_obj = match additional_error {
            None => {
                json!({ "error": base_error })
            }
            Some(a) => {
                json!({
                    "error": base_error,
                    "context": a
                })
            }
        };

        (status_code, Json(json_obj)).into_response()
    }
}

/// An enum intended to make it easier for endpoints that need to handle raw OR parsed
/// WebReg responses return a response.
pub enum RawParsedApiResp<T: Serialize> {
    Raw(webweg::types::Result<String>),
    Parsed(webweg::types::Result<T>),
}

impl<T> IntoResponse for RawParsedApiResp<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            RawParsedApiResp::Parsed(Err(e)) | RawParsedApiResp::Raw(Err(e)) => {
                ApiErrorType::from(e).into_response()
            }
            RawParsedApiResp::Parsed(Ok(o)) => (StatusCode::OK, Json(o)).into_response(),
            RawParsedApiResp::Raw(Ok(o)) => {
                let json = serde_json::from_str::<Value>(o.as_str());
                // Note: if we just returned Json(o) where o is of type String, then the response
                // will literally just be a giant string containing the JSON object. Users probably
                // expect to get the actual JSON structure itself, not the structure in a string.
                // So, we need to deserialize the JSON string to a JSON object and then return the
                // object.
                //
                // Two cases here: if we have a valid JSON structure from WebReg, we can
                // return the raw response as a JSON structure.
                //
                // If we do not have a valid JSON structure, then we can just return the original
                // string as is.
                match json {
                    Ok(o) => (StatusCode::OK, Json(o)).into_response(),
                    Err(_) => (StatusCode::OK, o).into_response(),
                }
            }
        }
    }
}

// https://serde.rs/enum-representations.html#untagged
#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum BodySearchType {
    SectionId {
        #[serde(rename = "sectionId")]
        section_id: String,
    },
    SectionIds {
        #[serde(rename = "sectionIds")]
        section_ids: Vec<String>,
    },
    SearchAdvanced {
        subjects: Option<Vec<String>>,
        courses: Option<Vec<String>>,
        departments: Option<Vec<String>>,
        instructor: Option<String>,
        title: Option<String>,
        #[serde(rename = "onlyOpen")]
        only_open: Option<bool>,
        #[serde(rename = "startHour")]
        start_hour: Option<i64>,
        #[serde(rename = "startMin")]
        start_min: Option<i64>,
        #[serde(rename = "endHour")]
        end_hour: Option<i64>,
        #[serde(rename = "endMin")]
        end_min: Option<i64>,
        days: Option<Vec<String>>,
        #[serde(rename = "levelFilter")]
        level_filter: Option<Vec<String>>,
    },
}

impl From<BodySearchType> for SearchType {
    fn from(value: BodySearchType) -> Self {
        match value {
            BodySearchType::SectionId { section_id } => SearchType::BySection(section_id),
            BodySearchType::SectionIds { section_ids } => {
                SearchType::ByMultipleSections(section_ids)
            }
            BodySearchType::SearchAdvanced {
                subjects,
                courses,
                departments,
                instructor,
                title,
                only_open,
                start_hour,
                start_min,
                end_hour,
                end_min,
                days,
                level_filter,
            } => {
                let mut search = SearchRequestBuilder::new();
                if let Some(s) = subjects {
                    search.subjects = s;
                }

                if let Some(c) = courses {
                    search.courses = c;
                }

                if let Some(d) = departments {
                    search.departments = d;
                }

                if let Some(i) = instructor {
                    search = search.set_instructor(i);
                }

                if let Some(t) = title {
                    search = search.set_title(t);
                }

                if let Some(o) = only_open {
                    search.only_open = o;
                }

                if let (Some(h), Some(m)) = (
                    start_hour.and_then(|h| u32::try_from(h).ok()),
                    start_min.and_then(|m| u32::try_from(m).ok()),
                ) {
                    search = search.set_start_time(h, m);
                }

                if let (Some(h), Some(m)) = (
                    end_hour.and_then(|h| u32::try_from(h).ok()),
                    end_min.and_then(|m| u32::try_from(m).ok()),
                ) {
                    search = search.set_end_time(h, m);
                }

                if let Some(d) = days {
                    for day in d {
                        match day.as_str() {
                            "M" | "m" => search = search.apply_day(DayOfWeek::Monday),
                            "Tu" | "tu" => search = search.apply_day(DayOfWeek::Tuesday),
                            "W" | "w" => search = search.apply_day(DayOfWeek::Wednesday),
                            "Th" | "th" => search = search.apply_day(DayOfWeek::Thursday),
                            "F" | "f" => search = search.apply_day(DayOfWeek::Friday),
                            "Sa" | "sa" => search = search.apply_day(DayOfWeek::Saturday),
                            "Su" | "su" => search = search.apply_day(DayOfWeek::Sunday),
                            _ => {}
                        }
                    }
                }

                if let Some(f) = level_filter {
                    for level in f {
                        match level.as_str() {
                            "l" | "L" => {
                                search = search.filter_courses_by(CourseLevelFilter::LowerDivision)
                            }
                            "u" | "U" => {
                                search = search.filter_courses_by(CourseLevelFilter::UpperDivision)
                            }
                            "g" | "G" => {
                                search = search.filter_courses_by(CourseLevelFilter::Graduate)
                            }
                            _ => {}
                        }
                    }
                }

                SearchType::Advanced(search)
            }
        }
    }
}
