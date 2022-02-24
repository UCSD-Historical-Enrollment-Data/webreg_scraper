use crate::{
    schedule::scheduler::Schedule,
    webreg::webreg_wrapper::{SearchRequestBuilder, SearchType, WebRegWrapper},
};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};

const SCHEDULE_FILE_NAME: &str = "schedule.txt";

/// Saves your proposed schedule to a file called `schedule.txt`.
///
/// # Parameters
/// - `s`: The schedules.
pub fn save_schedules(s: &[Schedule<'_>]) {
    let f = OpenOptions::new()
        .write(true)
        .create(true)
        .open(SCHEDULE_FILE_NAME)
        .expect("something went wrong when trying to create file.");

    let mut writer = BufWriter::new(f);
    s.iter().enumerate().for_each(|(s_num, s)| {
        s.sections.iter().enumerate().for_each(|(i, c)| {
            if i == 0 {
                // See [1]
                write!(writer, "{: >8} | ", s_num).unwrap();
                return;
            }

            if i == s.sections.len() - 1 {
                writeln!(
                    writer,
                    "{} {} {}",
                    c.subj_course_id, c.section_code, c.section_id
                )
                .unwrap();
                return;
            }

            write!(
                writer,
                "{} {} {} | ",
                c.subj_course_id, c.section_code, c.section_id
            )
            .unwrap();
        });
    });

    writer.flush().unwrap();
}

/// Puts all sections offered for a term into a CSV file so that it can be used for other applications.
///
/// # Parameters
/// - `w`: The `WebRegWrapper` reference.
pub async fn export_all_sections(w: &WebRegWrapper<'_>) {
    let file_name = format!("{}.csv", w.get_term());
    let f = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&file_name)
        .expect("something went wrong when trying to create file.");

    let mut writer = BufWriter::new(f);
    writeln!(
        writer,
        "subj_course_id,sec_code,sec_id,prof,total_seats,meetings"
    )
    .unwrap();

    // Empty builder so we can get all courses
    let s = SearchRequestBuilder::new();
    let results = w
        .search_courses(SearchType::Advanced(&s))
        .await
        .unwrap_or_default();

    for res in results {
        w.get_course_info(res.subj_code.trim(), res.course_code.trim())
            .await
            .unwrap_or_default()
            .into_iter()
            .for_each(|c| {
                writeln!(
                    writer,
                    "{},{},{},{},{},{}",
                    c.subj_course_id,
                    c.section_code,
                    c.section_id,
                    // Every instructor name (except staff) has a comma
                    c.instructor.replace(",", ";"),
                    c.total_seats,
                    c.meetings
                        .into_iter()
                        .map(|m| m.to_flat_str())
                        .collect::<Vec<_>>()
                        .join("|")
                )
                .unwrap();
            });
    }

    writer.flush().unwrap();
}

// References:
// [1] https://stackoverflow.com/questions/50458144/what-is-the-easiest-way-to-pad-a-string-with-0-to-the-left
