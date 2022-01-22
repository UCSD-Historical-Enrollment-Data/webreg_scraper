# ucsd_webreg_rs
A "wrapper" for UCSD's [WebReg](https://act.ucsd.edu/webreg2/start) course enrollment system.

## Programming Language & Libraries
This project uses the latest version of Rust (at this time, 1.58.1).

Additionally, the following libraries are used:
- `serde` / `serde_json`
- `reqwest`
- `tokio`

## Functionality
**Once finished**, this wrapper will be able to:
- Scrape basic WebReg information.
  - All courses that are offered for a given quarter.
  - Course data for a specific course (e.g. number of available seats).
- Plan and enroll in courses.

## Purpose
There are a few reasons why I wanted to make this wrapper:
- Monitor the enrollment count of certain classes (e.g. tracking popularity of certain classes).
- Automatically enroll in classes for me when possible (e.g. sniping open seats when available or when first/second pass is available).
- Create possible, conflict-free, schedules.

## Authentication
Originally, the biggest challenge I thought I would encounter was having to get around Duo. However, it turns out that using your cookies from a previous (authenticated) session will work (I'm not sure why it didn't work last time).

In order to use the wrapper, you only need to provide the cookie that is a part of the request header (for example, when loading a new page in WebReg).

In other words, this wrapper does not make use of an actual web scraper (WebReg is not a fun site to scrape).

## Why Rust?
To learn more about Rust, of course. Plus, I've been meaning to work on a project with Rust.

## Final Comment(s)
- This project is designed to be a somewhat long-term project. As such, it's expected to change quite a lot and is far from being done.
- This README will have more information when there is sufficiently more features.

## License
All code provided in this repository is licensed under the MIT license. 

As a warning, any data obtained from WebReg may have a different license.