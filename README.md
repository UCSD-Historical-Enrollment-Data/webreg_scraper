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
- [ ] Waitlisting a class.

### iii. Miscellaneous (Low-Priority)
- [x] Creating, or removing/renaming, a schedule.
- [ ] Adding, or removing, an event from a schedule.
- [x] Sending a confirmation email to yourself.

## Purpose
There are a few reasons why I wanted to make this wrapper:
- Monitor the enrollment count of certain classes (e.g. tracking popularity of certain classes).
- Automatically enroll in classes when possible (e.g. first/second pass is available).
- Create possible, conflict-free, schedules.

## Authentication

<details>
<summary>Minor Notes.</summary>
<br> 

Originally, one of the biggest challenge I thought I would encounter was having to get around Duo (the 2FA system we use). However, it turns out that using your cookies from a previous (active authenticated) session will work (I'm not sure why it did not work the last time I tried).

In order to use the wrapper, you only need to provide the cookie that is a part of the request header (for example, when loading a new page in WebReg). You can find this cookie by going to your developer tab, monitoring the requests that WebReg makes, and then getting your cookie from there.

</details>

## Remarks

<details>
<summary>Complaints about the API.</summary>
<br> 


WebReg's internal API is probably one of the messiest APIs I've ever used (which complements well with the fact that WebReg itself is an annoying website to scrape). Granted, it's not like we were *supposed* to use it in this fashion, but sometimes I wonder if the reason why it's this messy is just so people don't use their internal API by itself, like me.

One reason why the code I have is so verbose is because I want to clean the internal API's JSON responses. There are a lot of things that one needs to consider when using their API. I'll name two in particular.

### Specific Course Details in General
For example, suppose I wanted to fetch specific details about CSE 100 (number of people enrolled, professor teaching it, etc.). I would get a JSON array where *one* element in said array could either be:
- A JSON object representing a repeating MWF lecture. So, this object would say that the lecture occurs every MWF.
- A JSON object representing a repeating Thursday discussion. So, this object would say that the discussion occurs every Thursday.
- A JSON object representing a midterm. So, this object would say that the midterm occurs on Feb. 5, 2022.
- A JSON object representing a final. So, this object would say that the final occurs on Mar. 16.

Rather than giving me one giant JSON where each discussion has an associated lecture, midterm, and final exam, WebReg gives it to me as 4 separate entities. This might not seem terrible; however, let's consider a bigger example. Suppose I have to deal with 4 sections of Math 20C, each with 5 discussions, 2 midterms, a MWF lecture, and a final. Well, instead of giving me a JSON array with 20 elements (one element for each discussion and associated lecture/midterm/final), WebReg would give me **36 elements**:
- 20 discussion elements.
- 8 midterm elements.
- 4 lecture elements.
- 4 final elements.

So, I need to find some way to "group" all of these elements together so that I get the desired 20 elements. Another thing to mention is that the way WebReg labels meeting types like Lectures and Discussions is different from the way it labels Midterms/Finals (there's more work that needs to be done). I won't go too in-depth on that for now.

### Specific Course Details in Schedule
If you thought that the above was terrible, the internal WebReg API decided that it would be a wonderful idea to split any multiple-day repeated meetings into their own elements. So, if I had a MWF lecture, rather than giving me one JSON object representing a MWF lecture (like what you would expect *above*), WebReg gives it to me as three separate JSON objects; one object representing a Monday lecture, another representing a Wednesday lecture, and a third representing a Friday lecture.

Let's suppose I was enrolled in the CSE 100 section described above (so I would have a MWF lecture, Thursday discussion, and a set midterm and final date), and I wanted to get information on my enrolled classes. Rather than giving me an array of 4 JSON objects like how I described above (which is also what you would *at least* expect if WebReg's internal API was *consistent*), they instead decided to give me a JSON array consisting of:
- A JSON object representing a repeating Monday lecture. So, this object would say that the lecture occurs every Monday.
- A JSON object representing a repeating Wednesday lecture. So, this object would say that the lecture occurs every Wednesday.
- A JSON object representing a repeating Friday lecture. So, this object would say that the lecture occurs every Friday.
- A JSON object representing a repeating Thursday discussion. So, this object would say that the discussion occurs every Thursday.
- A JSON object representing a midterm. So, this object would say that the midterm occurs on Feb. 5, 2022.
- A JSON object representing a final. So, this object would say that the final occurs on Mar. 16.

So, if I had 4 classes each with a repeating MWF lecture, repeating one-day discussion, a midterm, and a final, I would have **28** separate elements that I would need to somehow group together.

On one hand, the reason why I think they did this is so it's easier for them to display the course information on a calendar.

As you can imagine, consistency isn't exactly something WebReg cares about. There's obviously a lot more that I can complain about, but I'll hold off on that for now.

</details>

## Disclaimer
I am not responsible for any damages or other issue(s) caused by any use of this wrapper. In other words, by using this wrapper, I am not responsible if you somehow get in trouble or otherwise run into problems.

## License
All code provided in this repository is licensed under the MIT license. 