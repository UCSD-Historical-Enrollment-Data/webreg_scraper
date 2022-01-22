mod webreg;
use crate::webreg::webreg::WebRegWrapper;
use std::error::Error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let w = WebRegWrapper::new("your cookies here", "WI22");
    let data = w.get_course_info("CSE", "12").await.unwrap();

    println!("{} possible sections found.", data.len());
    for d in data {
        println!("{}", d.to_string())
    }
    Ok(())
}
