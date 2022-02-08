use crate::{
    util::get_pretty_time,
    webreg::webreg_wrapper::{SearchRequestBuilder, WebRegWrapper},
};
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};

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
        "subj_course_id,sec_code,sec_id,prof,available,waitlist,total,meetings"
    )
    .unwrap();

    // Empty builder so we can get all courses
    let s = SearchRequestBuilder::new();
    let results = w.search_courses(&s).await.unwrap_or_default();

    for res in results {
        println!(
            "[{}] Processing: {} ({} {})",
            get_pretty_time(),
            res.course_title.trim(),
            res.subj_code.trim(),
            res.course_code.trim()
        );

        w.get_course_info(res.subj_code.trim(), res.course_code.trim())
            .await
            .unwrap_or_default()
            .into_iter()
            .for_each(|c| {
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{}",
                    c.subj_course_id,
                    c.section_code,
                    c.section_id,
                    // Every instructor name (except staff) has a comma
                    c.instructor.replace(",", ";"),
                    c.available_seats,
                    c.waitlist_ct,
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
