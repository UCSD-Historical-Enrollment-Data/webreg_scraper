use crate::util::{get_epoch_time, get_pretty_time};
use crate::{tracker, TermSetting, WebRegHandler};
use chrono::Local;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::iter::Sum;
use std::ops::Add;
use std::path::Path;
use std::time::Duration;
use tokio::sync::Mutex;
use webweg::webreg_wrapper::{SearchType, WebRegWrapper};

const CLEANED_CSV_HEADER: &str = "time,enrolled,available,waitlisted,total";

#[cfg(debug_assertions)]
const TIMEOUT: [u64; 3] = [5, 10, 15];

#[cfg(not(debug_assertions))]
// The idea is that it should take no more than 15 minutes for
// WebReg to be available.
const TIMEOUT: [u64; 3] = [8 * 60, 6 * 60, 4 * 60];

/// Runs the WebReg tracker. This will optionally attempt to reconnect to
/// WebReg when signed out.
///
/// # Parameters
/// - `w`: The wrapper.
/// - `s`: The wrapper handler.
pub async fn run_tracker(s: &WebRegHandler<'_>, end_loc: String) {
    // In case the given cookies were invalid, if this variable is false, we skip the
    // initial delay and immediately try to fetch the cookies.
    let mut first_passed = false;
    loop {
        tracker::track_webreg_enrollment(&s.scraper_wrapper, s.term_setting, &end_loc).await;

        // If we're here, this means something went wrong.
        if s.term_setting.recovery_url.is_none() {
            break;
        }

        // Basically, keep on trying until we get back into WebReg.
        let mut success = false;
        for time in TIMEOUT {
            if first_passed {
                println!(
                    "[{}] [{}] Taking a {} second break.",
                    s.term_setting.term,
                    get_pretty_time(),
                    time
                );
                tokio::time::sleep(Duration::from_secs(time)).await;
            }

            first_passed = true;

            // Get new cookies.
            let new_cookie_str = {
                match reqwest::get(s.term_setting.recovery_url.unwrap()).await {
                    Ok(t) => {
                        let txt = t.text().await.unwrap_or_default();
                        let json: Value = serde_json::from_str(&txt).unwrap_or_default();
                        if json["cookie"].is_string() {
                            Some(json["cookie"].as_str().unwrap().to_string())
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            };

            // And then try to make a new wrapper with said cookies.
            if let Some(c) = new_cookie_str {
                // Empty string = failed to get data.
                // Try again.
                if c.is_empty() {
                    continue;
                }

                s.scraper_wrapper.lock().await.set_cookies(c.clone());
                s.general_wrapper.lock().await.set_cookies(c);
                success = true;
                break;
            }
        }

        // If successful, we can continue pinging WebReg.
        if success {
            continue;
        }

        // Otherwise, gracefully quit.
        break;
    }

    println!(
        "[{}] [{}] Quitting.",
        s.term_setting.term,
        get_pretty_time()
    );
}

/// Tracks WebReg for enrollment information. This will continuously check specific courses for
/// their enrollment information (number of students waitlisted/enrolled, total seats) along with
/// basic course information and store this in a CSV file for later processing.
///
/// # Parameters
/// - `wrapper`: The wrapper.
/// - `setting`: The settings for this term.
/// - `end_location`: The end location for the cleaned CSV files. Just the base location will
///   suffice.
pub async fn track_webreg_enrollment(
    wrapper: &Mutex<WebRegWrapper<'_>>,
    setting: &TermSetting<'_>,
    end_location: &str,
) {
    // If the wrapper doesn't have a valid cookie, then return.
    if !wrapper.lock().await.is_valid().await {
        eprintln!(
            "[{}] [{}] Initial instance is not valid. Returning.",
            setting.term,
            get_pretty_time()
        );

        return;
    }

    let file_name = format!(
        "enrollment_{}_{}.csv",
        chrono::offset::Local::now().format("%FT%H_%M_%S"),
        setting.term
    );
    let is_new = !Path::new(&file_name).exists();
    // Map where the key is the course subj + number (e.g., CSE 30)
    // and the value is the associated CSV file which will be written to.
    let mut file_map: HashMap<String, File> = HashMap::new();

    let f = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_name)
        .unwrap_or_else(|_| panic!("could not open or create '{}'", file_name));

    let mut writer = BufWriter::new(f);
    if is_new {
        writeln!(
            writer,
            "time,subj_course_id,sec_code,sec_id,prof,available,waitlist,total,enrolled_ct"
        )
        .unwrap();
    }

    let mut fail_count = 0;
    'main: loop {
        writer.flush().unwrap();
        let w = wrapper.lock().await;
        let mut results = vec![];

        for search_query in &setting.search_query {
            let mut temp = w
                .search_courses(SearchType::Advanced(search_query))
                .await
                .unwrap_or_default();

            results.append(&mut temp);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Drop the Mutex to unlock it
        drop(w);

        if results.is_empty() {
            eprintln!(
                "[{}] [{}] No courses found. Exiting.",
                setting.term,
                get_pretty_time()
            );
            break;
        }

        println!(
            "[{}] [{}] Found {} results successfully.",
            setting.term,
            get_pretty_time(),
            results.len()
        );

        for r in results {
            if fail_count != 0 && fail_count > 12 {
                eprintln!(
                    "[{}] [{}] Too many failures when trying to request data from WebReg.",
                    setting.term,
                    get_pretty_time()
                );
                break 'main;
            }

            let w = wrapper.lock().await;
            let res = w
                .get_enrollment_count(r.subj_code.trim(), r.course_code.trim())
                .await;
            drop(w);

            match res {
                Err(e) => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] [{}] An error occurred ({}). Skipping. (FAIL_COUNT: {})",
                        setting.term,
                        get_pretty_time(),
                        e,
                        fail_count
                    );
                }
                Ok(r) if !r.is_empty() => {
                    fail_count = 0;
                    println!(
                        "[{}] [{}] Processing {} section(s) for {}.",
                        setting.term,
                        get_pretty_time(),
                        r.len(),
                        r[0].subj_course_id
                    );

                    let course_subj_id = r[0].subj_course_id.clone();
                    let time = get_epoch_time();
                    let time_formatted = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                    // Write to raw CSV dataset
                    r.iter().for_each(|c| {
                        writeln!(
                            writer,
                            "{},{},{},{},{},{},{},{},{}",
                            time,
                            c.subj_course_id,
                            c.section_code,
                            c.section_id,
                            // Every instructor name (except staff) has a comma
                            c.all_instructors.join(" & ").replace(',', ";"),
                            c.available_seats,
                            c.waitlist_ct,
                            c.total_seats,
                            c.enrolled_ct,
                        )
                        .unwrap()
                    });

                    // Pre-processing all of the files
                    let path_overall = Path::new(end_location)
                        .join(setting.term)
                        .join("overall")
                        .join(&format!("{}.csv", course_subj_id));

                    let exists_overall = path_overall.exists();

                    // Now write to the file
                    let file = file_map.entry(course_subj_id.clone()).or_insert_with(|| {
                        OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(&path_overall)
                            .unwrap_or_else(|_| {
                                panic!("could not open or create '{}'", path_overall.display())
                            })
                    });
                    // If the overall file doesn't exist, write the csv header
                    if !exists_overall {
                        writeln!(file, "{}", CLEANED_CSV_HEADER).unwrap();
                    }

                    // Calculate total seats and all of that here
                    let overall: CourseStat = r
                        .iter()
                        .map(|x| {
                            CourseStat(
                                x.enrolled_ct,
                                x.available_seats,
                                x.waitlist_ct,
                                x.total_seats,
                            )
                        })
                        .sum();

                    writeln!(
                        file,
                        "{},{},{},{},{}",
                        time_formatted, overall.0, overall.1, overall.2, overall.3
                    )
                    .unwrap();
                }
                _ => {
                    fail_count += 1;
                    eprintln!(
                        "[{}] [{}] Course {} {} not found. Were you logged out? (FAIL_COUNT: {}).",
                        setting.term,
                        get_pretty_time(),
                        r.subj_code,
                        r.course_code,
                        fail_count
                    );
                }
            }

            // Sleep between requests so we don't get ourselves banned by webreg
            tokio::time::sleep(Duration::from_secs_f64(setting.cooldown)).await;
        }
    }

    writer.flush().unwrap();
}

#[derive(Clone, Copy, Default)]
struct CourseStat(i64, i64, i64, i64);

impl Add<CourseStat> for CourseStat {
    type Output = CourseStat;

    fn add(self, rhs: CourseStat) -> Self::Output {
        CourseStat(
            self.0 + rhs.0,
            self.1 + rhs.1,
            self.2 + rhs.2,
            self.3 + rhs.3,
        )
    }
}

impl Sum for CourseStat {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|prev, next| prev + next).unwrap_or_default()
    }
}
