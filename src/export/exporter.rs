use crate::schedule::scheduler::Schedule;
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

// References:
// [1] https://stackoverflow.com/questions/50458144/what-is-the-easiest-way-to-pad-a-string-with-0-to-the-left
