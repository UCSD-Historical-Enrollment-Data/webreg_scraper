mod webreg;
use crate::webreg::webreg::WebRegWrapper;
use std::error::Error;

const COOKIE: &str = include_str!("../cookie.txt");

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let w = WebRegWrapper::new(COOKIE, "WI22");
    let data = w.get_course_info("CSE", "12").await.unwrap();

    println!("{} possible sections found.", data.len());
    for d in data {
        println!("{}", d.to_string())
    }
    Ok(())
}
