# webreg
A program designed to both "scrape" UCSD's WebReg for enrollment data and make the WebReg API available to other applications.

## Features
By default, this binary will constantly make requests to WebReg to get the number of students enrolled or waitlisted in
each class, along with the total number of seats for each section. These requests are made through your account, and the
session cookies are obtained through a helper `webregautoin` script.

This binary also includes a web server that automatically allows external applications to make requests to various WebReg 
endpoints; for example, external applications can make requests to the scraper's web server (which will forward that request
to WebReg) to get information like course information, prerequisites, and so on. Additionally, external applications can make
requests to said web server to _perform_ actions like enrolling in classes, dropping classes, and so on.

Note that the web server, by default, is unprotected. This means that anyone can make requests to it without needing to provide
authorization. This is good enough if you intend on using the web server locally only. However, if you want to make your web server 
accessible to the public, you can use the binary's `auth` feature, which sets up a very simple authorization system where external
applications must provide a bearer token when making a request to the server. To manage bearer tokens for the server, or to see
what authorization looks like, use the `authmanager` binary (or read more about it there).

**Note:** Starting with v0.5.0, the web server (including all WebReg endpoints) will be _bundled_ with the scraper. This design
choice was intentional. In previous versions of the binary, a web server has always been included (although, depending on the
executable type, it could be minimal or feature-packed). This has always required a bit of extra maintenance on my part, so
I've pretty much decided to just include everything going forward. If you want to use the scraper but do _not_ want the web
server, you'll need to manually modify the codebase to remove it.

## Setup
Below are instructions on how you can run this project. Please ensure that you've set the `webregautoin` script up, 
and that it's running, before continuing.

### Using Pre-Compiled Executable

1. Download the [relevant executable here](https://github.com/ewang2002/webreg_scraper/releases). The executables will 
   be in an archive whose name is of the form `webreg-[target]-[features]`. Here,
   - `[target]` is the target operating system that you can run the executable for. At this time, we have
     - `x86_64-apple-darwin`: MacOS
     - `x86_64-pc-windows-msvc`: Windows
     - `x86_64-unknown-linux-gnu`: Linux
   - `[features]` is the features included in this binary. At this time, we have
     - `default`: Only the scraper and web server are included.
     - `auth`: The scraper and web server (which includes basic authentication) are included.
2. A sample configuration file has been provided for you; this file is called `config.example.json`.
   1. Rename this file to `config.json`.
   2. Modify the configuration information appropriately. Information on the structure of the configuration file can be
      found below.
   3. Save your changes.

3. Extract the executable from the archive.
4. Run the executable like so,
   ```
   ./webreg <path_to_config_file>
   ```
   where `<path_to_config_file>` is the name of your configuration file (assuming it's in the same directory as the
   executable).

### Self-Compiling Executable
If you're interested in self-compiling, follow the instructions below.

1. You'll need to install [Rust](https://www.rust-lang.org/tools/install). If you're on Ubuntu (or a similar system),
   you'll need to install the following system dependencies:
   ```
   sudo apt-get update
   sudo apt install build-essential
   sudo apt install pkg-config
   sudo apt install libssl-dev
   ```

2. Clone, or download, this repository.
3. You can now compile the project. 

   If you'd like both the scraper and web server, without authentication, run the command
   ```
   cargo build --release --bin webreg
   ```

   If you'd like the scraper and web server _with_ basic authentication, run the command
   ```
   cargo build --release --bin webreg --features auth
   ```
4. You should find the `webreg` executable in the `/target/release` directory. Under the "Using Pre-Compiled Executable"
   section, follow step 2 to set your configuration file up, and step 4 to run the executable.

## Configuration File
In order to run this binary, you'll need to provide a configuration file. Below, you'll get an idea of what the configuration 
file should look like. All entries are required. For an example of this configuration file, check out `config.example.json`.

### Base (Root Object)
All information below will be in the root object.

| Key | Type | Information |
| --- | ---- | ----------- |
| `configName` | `string` | The name of the configuration file. This is only used for identification purposes. |
| `apiBaseEndpoint` | `object` | Hosting information for the web server for the API. See **API Info / Recovery Info** for associated entries. |
| `cookieServer` | `object` | The address to the web server that the scraper can use to log back into WebReg if it gets logged out. See **API Info / Recovery Info** for more information. This relies on [`webregautoin`](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin).  |
| `verbose` | `boolean` | Whether logging should be verbose. |
| `wrapperData` | `object[]` | An array of objects representing each term that the scraper should consider. See **Wrapper Data** for associated entries. |

### Base → API Info / Recovery Info
All entries below are under `apiBaseEndpoint`.

| Key | Type | Information |
| --- | ---- | ----------- |
| `address` | `string` | The web server's address. |
| `port` | `number` | The web server's port. |

### Base → Wrapper Data
All entries below are under `wrapperData`.

| Key | Type | Information |
| --- | ---- | ----------- |
| `term` | `string` | The four character term that the scraper should consider. The first two characters must be one of `FA`, `WI`, `SP`, `S1`, `S2`, `S3` and the last two characters must be an integer representing the year. For example, `SP24` represents the `Spring 2024` term. |
| `cooldown` | `number` | The cooldown between requests, in seconds. |
| `searchQuery` | `object[]` | The courses to search and gather data for. See **Search Query** for associated entries. |
| `saveDataToFile` | `boolean` | Whether the data scraped for this term is actually saved. **At the moment, this is _not_ being used.** |

### Base → Wrapper Data → Search Query
All entries below are under `wrapperData[n].searchQuery`, where `n` is some integer used to index the array.

| Key | Type | Information |
| --- | ---- | ----------- |
| `levels` | `string[]` | The course levels. This can either be `g` (graduate), `u` (upper-division), or `l` (lower-division) |
| `departments` | `string[]` | All departments to consider. All elements here must be the department's code (e.g., for all courses under the History department, use `HIST`). An empty array indicates that all departments should be considered. |


## Implementation
I'll only focus on the program's main feature -- tracking enrollment counts.

<details>
<summary>More Information</summary>
<br> 


### The Idea
At a very high level, the program (specifically, the scraper part) runs the following in an endless loop:
- Retrieve all possible courses.
- For each course:
    - Request data for that course.
    - Save that data to the CSV file.

We make use of [green threads](https://docs.rs/tokio/latest/tokio/task/index.html), managed by the Tokio runtime, to run through the above loop **concurrently** with other terms. In other words, we can say that we're running the above "program" multiple times at the same time.

### Authentication
How do we run the above "program?" Well, the program makes use of an internal API that only UCSD students have access to (this is the same API that WebReg uses). To access the internal API, the program needs to be "logged into WebReg." This usually isn't hard; under most circumstances, you can just log in by "simulating" the login process. This is usually done in two ways:
- When sending the request, include an API key.
- Have the program make some API call with your login credentials and then retrieve the session cookies, which can then be used for future requests.

Now, it's obvious that WebReg (or UCSD) isn't going to give me an API key. Regarding the second point, like with most schools, we have a 2FA system (Duo), which prevents me from just "simulating" the login process. More specifically, this is because due to two reasons:
- The big reason: most HTTP clients (like `reqwest`, which is what we're using) can only load the initial page source (what you would see when you view the page's source). However, Duo *dynamically* loads in a JavaScript `iframe`, which HTTP clients cannot render. Since it cannot render, the HTTP client can never "answer" it.
- A smaller reason: even if we *could* render the 2FA prompt, we would have to **manually** input the information (a 2FA code, for example).

Therefore, the solution is to just manually log in ourselves and then give the program the session cookies before running it.

### The Challenge
So, that simple solution doesn't seem too bad. However, the challenging part is keeping the program running 24/7. You see, WebReg restarts at around 4:30 AM every day. When WebReg restarts, every active session gets logged out. This includes the program.

The obvious solution would be to wake up at 4:30 AM every day and manually log the program in. However, this itself brings another problem -- waking up that early is hard for me.

This brings me to another solution -- there is additionally another project, creatively named [`webregautoin`](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin), which uses Node's [HTTP](https://nodejs.org/api/http.html) library to create a local web server which the wrapper can use. In particular, this local web server has one sole purpose: when new session cookies are needed to log into WebReg, the wrapper can make a request to the local API. The local API will then use [a headless Chrome browser](https://github.com/puppeteer/puppeteer) to log into WebReg and get the new cookies. Afterwards, those new cookies are returned back to the requester (the program), which the program can use to log back into WebReg and make more requests.

Why does *this* work but not the HTTP client? Well, a *headless browser* acts just like any browser, which means we can *load* Duo's `iframe` and thus authenticate. Of course, 2FA usually requires the end user to manually input a code. However, we can tell Duo to remember the current browser for 7 days*. Therefore, as long as the browser isn't closed\*\*, the headless browser can "bypass" the 2FA prompt and go straight to WebReg, which means we can get the session cookies.


\* - Note that if we couldn't do this, then we would have to resort to waking up every day at 4:30 AM. Oh, the horrors...

\*\* - Closing and reopening our headless browser resets everything due to the way most headless browsers are implemented.

</details>