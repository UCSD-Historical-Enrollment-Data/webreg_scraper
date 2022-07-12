<p align="center">
  <img src="https://github.com/ewang2002/webreg_scraper/blob/master/assets/project_banner.png?raw=true"  alt="Project Banner"/>
</p>

<p align="center">
  <a href="https://github.com/ewang2002/webweg">webweg</a> |
  <b>webreg_scraper</b> |
  <a href="https://github.com/ewang2002/UCSDHistEnrollData">UCSDHistEnrollmentData</a>
</p>

A program designed to scrape WebReg for enrollment data.

## Programming Language
This scraper uses the latest version of the [Rust](https://www.rust-lang.org/) programming language.

There is, additionally, another mini-project which uses the TypeScript programming language.

## Features
This program's main feature is that it can continuously track enrollment counts and save this information to a CSV file, which can then be used to analyze how fast classes fill up. Note that these CSV files need to be further processed (they are considered *raw* CSV files).

Some other features that are supported include:
- creating conflict-free schedules (not exactly efficiently, but it works),
- exporting all available sections into a CSV file, and
- hosting a small web server (using [Rocket](https://rocket.rs/)) to make it easy for other applications to use the data.

## Conditional Compilation
If you'd like the program to commit the (cleaned) CSV files directly to a repository every 5 minutes, enable the `git_repeat` flag. **By default**, this is not the case.

You will need to create a `clean.txt` file with the content only being a path to the directory containing the Git repository information (i.e., the `.git` folder). The target directory itself can be empty; the program will create the necessary folders.

As a side note, there appears to be a bug with either the program or Git where some files are not being properly pushed to GitHub.

## Setup
Below are instructions on how you can build and run this project.

### Required & Optional Software
Make sure to get the latest versions of the required software.

#### Required
- [Rust](https://www.rust-lang.org/).

#### Optional
- [git](https://git-scm.com/).

### Instructions
0. If you're on Ubuntu, you will need to install some packages. Run the following commands.
    ```
    sudo apt-get update
    sudo apt install build-essential
    sudo apt install pkg-config
    sudo apt install libssl-dev
    ```

    If you're on a different distribution, you will need to find the equivalent commands on your own.

1. Download the contents of this repository. You can either download it directly, or use something like `git clone`.
2. Next, in the base folder (the one containing all of the term folders and the scripts), open a terminal of your choice and run
    ```
    cargo build --release
    ```
3. You should find the executable in the `/target/release` directory.

## Implementation
I'll only focus on the program's main feature -- tracking enrollment counts.

<details>
<summary>More Information</summary>
<br> 


### The Idea
At a very high level, the program runs the following in an endless loop:
- Retrieve all possible courses.
- For each course:
    - Request data for that course.
    - Save that data to the CSV file.

We make use of [green threads](https://docs.rs/tokio/latest/tokio/task/index.html), managed by the Tokio runtime, to run through the above loop **concurrently** with other terms. In other words, we're running the above "program" multiple times at the same time. 

### Authentication
How do we run the above "program?" Well, the program makes use of an internal API that only UCSD students have access to (this is the same API that WebReg uses). To access the internal API, the program needs to be "logged into WebReg." This usually isn't hard; under most circumstances, you can just log in by "simulating" the login process. This is usually done in two ways:
- When sending the request, include an API key.
- Have the program make some API call with your login credentials and then retrieve the session cookies, which can then be used for future requests. 

Now, it's obvious that WebReg isn't going to give me an API key. Regarding the second point, like with most schools, we have a 2FA system (Duo), which prevents me from just "simulating" the login process. More specifically, this is because due to two reasons:
- The big reason: most HTTP clients (like `reqwest`, which is what we're using) can only load the inital page source (what you would see when you view the page's source). However, Duo *dynamically* loads in a JavaScript `iframe`, which HTTP clients cannot render. Since it cannot render, the HTTP client can never "answer" it.
- A smaller reason: even if we *could* render the 2FA prompt, we would have to **manually** input the information (a 2FA code, for example).

Therefore, the solution is to just manually log in ourselves and then give the program the session cookies before running it.

### The Challenge
So, that simple solution doesn't seem too bad. However, the challenging part is keeping the program running 24/7. You see, WebReg restarts at around 4:30 AM every day. When WebReg restarts, every active session gets logged out. This includes the program.

The obvious solution would be to wake up at 4:30 AM every day and manually log the program in. However, this itself brings another problem -- waking up that early is hard for me.

This brings me to another solution -- there is additionally another project, creatively named [`webregautoin`](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin), which uses Node's [HTTP](https://nodejs.org/api/http.html) library to create a local API server which the wrapper can use. In particular, this local API has one sole purpose: when new session cookies are needed to log into WebReg, the wrapper can make a request to the local API. The local API will then use [a headless Chrome browser](https://github.com/puppeteer/puppeteer) to log into WebReg and get the new cookies. Afterwards, those new cookies are returned back to the requester (the program), which the porgram can use to log back into WebReg and make more requests.

Why does *this* work but not the HTTP client? Well, a *headless browser* acts just like any browser, which means we can *load* Duo's `iframe` and thus authenticate. Of course, 2FA usually requires the end user to manually input a code. However, we can tell Duo to remember the current browser for 7 days*. Therefore, as long as the browser isn't closed\*\*, the headless browser can "bypass" the 2FA prompt and go straight to WebReg, which means we can get the session cookies.


\* - Note that if we couldn't do this, then we would have to resort to waking up every day at 4:30 AM.
\*\* - Closing and reopening our headless browser resets everything due to the way most headless browsers are implemented.

</details>


## License
Everything in this repository is licensed under the MIT license.