# ucsd_webreg_rs
An API wrapper for UCSD's [WebReg](https://act.ucsd.edu/webreg2/start) course enrollment system.

## Programming Language
The main project (API wrapper) uses the latest version of [Rust](https://www.rust-lang.org/). The reason why I chose Rust instead of, say, Python or C#, is because I wanted to learn more about Rust. Plus, I've been meaning to work on a project with Rust.

There is additionally another project, creatively namd `webregautoin`, which uses Node's [HTTP](https://nodejs.org/api/http.html) library to create a local API server which the wrapper can use. In particular, this local API has one sole purpose: when new cookies are needed to log into WebReg, the wrapper can make a request to the local API. The local API will then use [a headless Chrome browser](https://github.com/puppeteer/puppeteer) to log into WebReg and get the new cookies. Note that you'll need to log into WebReg beforehand so you can select the `Remember me for 7 days` checkbox for the Duo 2FA (this will automatically be done when an initial request is made).

## Wrapper Features (WIP & Completed)
Below are some features that this wrapper has, along with what I plan on working on.

### i. General GET Requests
- [x] Get all possible classes in the quarter.
- [x] Searching for classes based on some conditions (Advanced Search).
- [x] Get detailed information about a specific class (like number of students enrolled, section times, etc.)
- [x] Getting your current schedule.
- [x] Ability to get detailed information for a wide range of classes.

### ii. General POST Requests
- [x] Changing grading option.
- [x] Enrolling in, or dropping, a class.
- [x] Planning, or un-planning, a class.
- [x] Waitlisting a class.

### iii. Miscellaneous (Low-Priority)
- [x] Creating, or removing/renaming, a schedule.
- [ ] Adding, or removing, an event from a schedule.
- [x] Sending a confirmation email to yourself.

## Purpose
There are a few reasons why I wanted to make this wrapper:
- Monitor the enrollment count of certain classes (e.g. tracking popularity of certain classes).
- Automatically enroll in classes when possible (e.g. first/second pass is available).
- Create possible, conflict-free, schedules.

## Wrapper Usage
To use the wrapper, you need to create a new instance of it. For example:
```rs
let term = "SP22";
let cookie = "your authorization cookies here";
let w = WebRegWrapper::new(cookie.to_string(), term);
```

Where the cookies are your authentication cookies (which you can find by looking at the cookie options in the header of any WebReg API request under the `Network` tab in Developer Tools). 

Once created, you're able to use the various wrapper functions. Some useful examples are shown below (note that `w` refers to the declaration above).

### Check Login Status
You can check to see if you are logged in (i.e. if the wrapper can actually perform any useful requests). 

<details>
<summary>General Example</summary>
<br> 

```rs
if !w.is_valid().await {
    println!("You aren't logged in!");
    return; 
}
```

</details>



### Get Schedule
You can get your current schedule, which lists your Enrolled, Planned, and Waitlisted courses. You are able to fetch either the default schedule (`None`) or a specific schedule (e.g. `My Schedule 2`)

<details>
<summary>General Example</summary>
<br> 
Suppose you wanted to see what courses are currently in your *default* schedule. We can use the following code:

```rs
let my_schedule = w.get_schedule(None).await;
if let Some(schedule) = my_schedule {
    for s in schedule {
        println!("{}", s.to_string());
    }
}
```

This prints out:
```
[A05 / 75220] Ethics And Society II (POLI 28) with Elgin, Samuel Zincke - Enrolled (4 Units, L Grading, 21 / 34)
        [LE] M at 12:00 - 12:50 in CENTR 101
        [LE] W at 12:00 - 12:50 in CENTR 101
        [FI] 2022-06-08 at 11:30 - 14:29 in CENTR 101
        [DI] F at 12:00 - 12:50 in SEQUO 148

... (other courses not listed)
```

**Remark:** If you wanted to see what courses you have planned in some other schedule, you can replace `None` with `Some("your schedule name here")`. 
</details>


### Get Course Information
You are able to search up course information for a particular course. If no authentication issues occur, then this function will return a vector where each element contains the instructor name, number of seats, and all meetings.  

<details>
<summary>General Example</summary>
<br> 
Suppose we wanted to look up all CSE 101 sections. We can use the following code:

```rs
let courses_101 = w.get_course_info("CSE", "101").await;
if let Some(courses) = courses_101 {
    for c in courses {
        println!("{}", c.to_string());
    }
}
```

This prints out:
```
[CSE 101] [A01 / 079914] Dasgupta, Sanjoy: 0/116 (WL: 0)
        [LE] TuTh at 9:30 - 10:50 in CENTR 119
        [DI] F at 15:00 - 15:50 in CENTR 119
        [FI] 2022-06-04 at 11:30 - 14:29 in WLH 2001

[CSE 101] [B01 / 079915] Impagliazzo, Russell: 0/116 (WL: 0)
        [LE] TuTh at 14:00 - 15:20 in WLH 2005
        [DI] F at 16:00 - 16:50 in CENTR 119
        [FI] 2022-06-04 at 11:30 - 14:29 in WLH 2005
```

</details>

### Search Courses
You can also search up courses that meet a particular criteria. This is very similar in nature to the Advanced Search option.

<details>
<summary>Example: Searching by Section(s)</summary>
<br> 
Suppose we wanted to search for specific sections. In our example below, we'll search for one section of CSE 100, one section of Math 184, and one section of POLI 28. The following code will do just that: 

```rs
let search_res = w
    .search_courses_detailed(SearchType::ByMultipleSections(&[
        "079913", "078616", "075219",
    ]))
    .await;
if let Some(res) = search_res {
    for r in res {
        println!("{}", r.to_string());
    }
}
```

This prints out:
```
[CSE 100] [B02 / 079913] Staff: 0/68 (WL: 0)
        [LE] MWF at 10:00 - 10:50 in CENTR 119
        [DI] W at 17:00 - 17:50 in CSB 002
        [FI] 2022-06-04 at 8:00 - 10:59 in WLH 2005

[MATH 184] [A03 / 078616] Kane, Daniel Mertz: 27/35 (WL: 0)
        [LE] MWF at 16:00 - 16:50 in HSS 1330
        [DI] Th at 19:00 - 19:50 in APM 7321
        [FI] 2022-06-09 at 15:00 - 17:59 in HSS 1330

[POLI 28] [A04 / 075219] Elgin, Samuel Zincke: 26/34 (WL: 0)
        [LE] MW at 12:00 - 12:50 in CENTR 101
        [DI] W at 16:00 - 16:50 in SOLIS 111
        [FI] 2022-06-08 at 11:30 - 14:29 in CENTR 101
```

</details>

<details>
<summary>Example: Searching by Criteria</summary>
<br> 

Suppose we wanted to search for any lower- or upper-division CSE course. We can use the following code:

```rs 
let search_res = w
    .search_courses_detailed(SearchType::Advanced(
        &SearchRequestBuilder::new()
            .add_department("CSE")
            .filter_courses_by(CourseLevelFilter::UpperDivision)
            .filter_courses_by(CourseLevelFilter::LowerDivision),
    ))
    .await;

if let Some(r) = search_res{
    for c in r {
        println!("{}", c.to_string());
    }
}
```

This prints out:
```
[CSE 6R] [A01 / 077385] Moshiri, Alexander Niema: 14/150 (WL: 0)
        [LE] MWF at 11:00 - 11:50 in RCLAS R05
        [DI] W at 12:00 - 12:50 in RCLAS R05
        [MI] 2022-04-30 at 10:00 - 10:50 in RCLAS R05

... (other courses not listed)

[CSE 185] [A03 / 077491] Gymrek, Melissa Ann: 34/38 (WL: 0)
        [LE] MW at 11:00 - 11:50 in CENTR 105
        [LA] MW at 13:00 - 14:50 in EBU3B B270
```

</details>



### Enroll in Section
You can use the wrapper to plan or enroll in a particular section. 

<details>
<summary>Planning a Course</summary>
<br> 
Suppose you wanted to plan a section of CSE 100 to your default schedule. You can use the following code:

```rs
w.add_to_plan(PlanAdd {
    subject_code: "CSE",
    course_code: "100",
    section_number: "079911",
    section_code: "A01",
    // Using S/U grading.
    grading_option: Some("S"),
    // Put in default schedule
    schedule_name: None,
    unit_count: 4
}, true).await;
```

This will return `true` if the planning succeeded and `false` otherwise.

**Remark:** If you wanted to see what courses you have planned in some other schedule, you can replace `None` with `Some("your schedule name here")`. 

</details>

<details>
<summary>Unplanning a Course</summary>
<br> 
Suppose you want to remove the section of CSE 100 from your default schedule. You can use the following code:

```rs
w.remove_from_plan("079911", None).await;
```

This will return `true` if the removal succeeded and `false` otherwise.

**Remark:** If you wanted to see what courses you have planned in some other schedule, you can replace `None` with `Some("your schedule name here")`. 

</details>

<details>
<summary>Enrolling in, or Waitlisting, a Course</summary>
<br> 
Suppose your enrollment time is here and you want to enroll/waitlist in a specific section of CSE 95. You can use the following code:

```rs
w.add_section(
    // To waitlist, use `false` instead.
    true,
    EnrollWaitAdd {
        // All you need is a section ID
        section_number: "078483",
        // Using the default grading option
        grading_option: None,
        // And the default unit count
        unit_count: None,
    },
    true
).await;
```

This will return `true` if you were able to enroll/waitlist in the section and `false` otherwise. Additionally, if you are able to enroll/waitlist in said section, this function will also call an API endpoint which unplans said class from all of your schedules.

</details>

<details>
<summary>Dropping a Course</summary>
<br> 
Suppose your enrollment time is here and you decide to drop CSE 95. You can use the following code:

```rs
// If this course is on the waitlist, use `false` instead.
w.drop_section(true, "078483").await;
```

This will return `true` if dropping was successful and `false` otherwise.


</details>




## Disclaimer
I am not responsible for any damages or other issue(s) caused by any use of this wrapper. In other words, by using this wrapper, I am not responsible if you somehow get in trouble or otherwise run into problems.

## Want Data?
Specifically, how fast a lower- or upper-division CSE/COGS/MATH/ECE course fills up in Spring 2022? [Here you go.](https://github.com/ewang2002/UCSDHistEnrollData)

## License
All code provided in this repository is licensed under the MIT license. 