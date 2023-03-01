<p align="center">
  <img src="https://github.com/ewang2002/webreg_scraper/blob/master/assets/project_banner.png?raw=true"  alt="Project Banner"/>
</p>

<p align="center">
  <a href="https://github.com/ewang2002/webweg">webweg</a> |
  <b>webreg_scraper</b> |
  <a href="https://github.com/ewang2002/UCSDHistEnrollData">UCSDHistEnrollmentData</a>
</p>

A program designed to both scrape WebReg for enrollment data and make WebReg data available to other applications.

**NOTE:** If you just want a list of all offered courses, including all meeting information, in an active term, use the [utility program](https://github.com/ewang2002/Utilities/tree/master/wgtools) instead. This scraper only handles the number of students enrolled in a course.

# Table of contents

- [Features](#features)
- [Helpers](#helpers)
- [Scraper Configuration File](#scraper-configuration-file)
    - [Base](#base)
    - [Base → API Info / Recovery Info](#base--api-info--recovery-info)
    - [Base → Wrapper Data](#base--wrapper-data)
    - [Base → Wrapper Data → Search Query](#base--wrapper-data--search-query)
- [Sample Configuration File](#sample-configuration-file)
- [Setup](#setup)
    - [Part 1: `webregautoin`](#part-1-webregautoin)
    - [Part 2: The Scraper](#part-2-the-scraper)
- [Implementation](#implementation)
    - [The Idea](#the-idea)
    - [Authentication](#authentication)
    - [The Challenge](#the-challenge)
- [License](#license)

Generated by [this website](https://luciopaiva.com/markdown-toc/).

## Features
This program has two main features:

| Feature | Description  | 
| ------- | ------------ | 
| `api`   | Exposes the main parts of WebReg's API for other applications to freely use, via a web framework (Axum). |
| `scraper` | Tracks enrollment counts using WebReg's API. |

Each feature can be enabled individually. That is, if you only want the actual _scraper_ part, you can compile the project 
so that the API part is not compiled.

## Helpers
Aside from the scraper itself, there are two other helper programs -- one of which is essentially required.

| Program | Information |
| ------- |-------------|
| [`webregautoin`](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin) | A basic web server designed to automatically log the scraper into WebReg. **This is required.** |
| [`notifierbot`](https://github.com/ewang2002/webreg_scraper/tree/master/notifierbot) | A simple Discord Bot that notifies you if the scraper is not working. |

This program _requires_ the `webregautoin` helper program. 

## Scraper Configuration File
Below, you'll get an idea of what the configuration file should look like.

### Base
| Key | Type | Information | Required | Used By |
| --- |------|-------------| ------ |---------|
| `configName` | `string` | The name of the configuration file. This is only used for identification purposes. | Y | all |
| `apiInfo` | `object` | Hosting information for the web server for the API. See **API Info / Recovery Info** for associated entries. | Y | api |
| `verbose` | `boolean` | Whether logging should be verbose. | Y | all |
| `wrapperData` | `object[]` | An array of objects representing each term that the scraper should consider. See **Wrapper Data** for associated entries. | Y | all |

### Base → API Info / Recovery Info
All entries below are under `apiInfo`.

| Key | Type | Information | Required | Used By |
| --- |------|-------------| ------ |---------|
| `address` | `string` | The web server's address. | Y | api |
| `port` | `number` | The web server's port. | Y | api |

### Base → Wrapper Data
All entries below are under `wrapperData`.

| Key | Type | Information | Required | Used By |
| --- |------|-------------| ------ |---------|
| `term` | `string` | The four character term that the scraper should consider. The first two characters must be one of `FA`, `WI`, `SP`, `S1`, `S2`, `S3` and the last two characters must be an integer representing the year. For example, `SP24` represents the `Spring 2024` term. | Y | all |
| `cooldown` | `number` | The cooldown between requests, in seconds. | Y | scraper |
| `searchQuery` | `object[]` | The courses to search and gather data for. See **Search Query** for associated entries. | Y | scraper |
| `applyBeforeUse` | `boolean` | Whether the force the scraper to apply the term to it before scraping data. This is useful when, for example, the term is valid (i.e., it is in the WebReg system) but cannot be accessed through normal means. | Y | all |
| `alias` | `string` | The term alias. This is used in the file name for the CSV files representing the gathered data. | N | scraper |
| `recoveryInfo` | `object` | The address to the web server that the scraper can use to log back into WebReg if it gets logged out. See **API Info / Recovery Info** for more information. | Y | all |

### Base → Wrapper Data → Search Query
All entries below are under `wrapperData[n].searchQuery`, where `n` is some integer used to index the array.

| Key | Type | Information | Required | Used By |
| --- |------|-------------| ------ |---------|
| `levels` | `string[]` | The course levels. This can either be `g` (graduate), `u` (upper-division), or `l` (lower-division) | Y | scraper |
| `departments` | `string[]` | All departments to consider. All elements here must be the department's code (e.g., for all courses under the History department, use `HIST`). An empty array indicates that all departments should be considered. | Y | scraper |

## Sample Configuration File
The following configuration file (also found in `config.example.json`) defines the following:
- Simply named **Production Configuration**.
- Create a web server on `0.0.0.0:3000`.
- Does not allow verbose logging.
- Defines one term (`WI23`, or Winter 2023) with the following information:
  - After a request, wait 0.5 seconds before making another request.
  - If the scraper gets logged out, use the web server at `127.0.0.1:3001` to get new session cookies.
    - This relies on [`webregautoin`](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin). 
  - Scrape the following courses:
    - All graduate MATH, CSE, COGS, and ECE courses.
    - All lower-division and upper-division courses.
  - No need to tell the scraper to forcibly change the term to the Winter 2023 term. 

```json
{
  "configName": "Production Configuration",
  "apiInfo": {
    "address": "0.0.0.0",
    "port": 3000
  },
  "verbose": false,
  "wrapperData": [
    {
      "term": "WI23",
      "cooldown": 0.5,
      "recoveryInfo": {
        "address": "127.0.0.1",
        "port": 3001
      },
      "searchQuery": [
        {
          "levels": ["g"],
          "departments": ["MATH", "CSE", "COGS", "ECE"]
        },
        {
          "levels": ["l", "u"],
          "departments": []
        }
      ],
      "applyBeforeUse": false
    }
  ]
}
```

## Setup
Below are instructions on how you can build and run this project. This setup guide assumes the use of Ubuntu for the host operating system, although other Linux distributions should be relatively similar.

First, please install the latest version of:
- [Node.js](https://nodejs.org/en/)
- [Rust](https://www.rust-lang.org/)
  - if you want to compile the project yourself; if you just want the executables, they are provided in the Releases section. More on this below.

### Part 1: `webregautoin`
As mentioned above, `webregautoin` is the script that allows the scraper to actually log into WebReg.
1. A sample configuration file has been provided for you; this file is called [`credentials.example.json`](https://github.com/ewang2002/webreg_scraper/blob/master/webregautoin/credentials.sample.json). 
   1. Rename this file to `credentials.json`. 
   2. Open the file and fill in your UC San Diego Active Directory username and password. 
   3. Save your changes. 

2. Install the required system dependencies:
    ```
    sudo apt-get update
    sudo apt install libgtk-3-dev libnotify-dev libgconf-2-4 libnss3 libxss1 libasound2
    ```
    This was taken from [here](https://github.com/puppeteer/puppeteer/blob/main/docs/troubleshooting.md#running-puppeteer-on-wsl-windows-subsystem-for-linux).    

3. Next, install TypeScript globally:
    ```
    npm i -g typescript 
    ```
    As the script is written in TypeScript, you need to install TypeScript to "compile" the script.

4. Afterwards, install the dependencies that this script needs using the command:
    ```
    npm i
    ```

5. Run the following command to compile the script:
    ```
    npm run compile
    ```
    This is an alias for the command `tsc -p .`, which is defined in the `package.json` file.

6. At this point, you should see an `out` folder. In the `out` folder, you should have an `index.js` file. You can 
   run the command like so:
    ```
    node index.js <term> <port>
    ```
   where 
    - `term` is the term that you want to scrape data for, or otherwise make data accessible to other apps. This 
       must be four character. 
      - The format is as follows:
        - The first two characters must be one of
          - `FA` for Fall, or
          - `WI` for Winter, or
          - `SP` for Spring, or
          - `S1` for Summer Session 1, or
          - `S2` for Summer Session 2, or
          - `S3` for Summer Session 3.
        - The last two characters must be the year associated with the term. For example, `21` means 2021.
      - For example:
        - `FA22` represents the Fall 2022 term.
        - `WI23` represents the Winter 2023 term.
    - `port` is the port where this script should be "visible" to the scraper. Usually, I put a number like `3001` or
      `4000`.

There are a few things to keep in mind:
- Each term must have its own instance of this application. So, if you have two terms that you want to use for the
  scraper, you must run this application in parallel twice.
- You must redo this process once every 6-7 days, since Duo only remembers you for up to 7 days. 

### Part 2: The Scraper

Please follow the directions based on what you want. 

<details>
<summary>Just Want Executable.</summary>
<br> 

If you just want the executable, download the [relevant executable here](https://github.com/ewang2002/webreg_scraper/releases). The executables are in a zipped file, so you'll need to unzip it. The naming scheme of the file is `webreg_scraper-x86_64-[operating system]-[features].[extension]`, where you can find the [features here](#features). If you want all features, then download the archive with `all` in its name. 

After you get the relevant executable, please finish step 3 and the later part of step 5 under "Self-Compiling." 

</details>


<details>
<summary>Self-Compiling.</summary>
<br> 

1. Install the required system dependencies:
    ```
    sudo apt-get update
    sudo apt install build-essential
    sudo apt install pkg-config
    sudo apt install libssl-dev
    ```

2. Download the contents of this repository. You can either download it directly, or use something like `git clone`.

3. A sample configuration file has been provided for you; this file is called [`config.example.json`](https://github.com/ewang2002/webreg_scraper/blob/master/config.example.json). 
   1. Rename this file to `config.json`. 
   2. Modify the configuration information appropriately. Information on the structure of the configuration file can be found [above](#scraper-configuration-file).
   3. Save your changes. 

4. Now, you'll have the opportunity to compile the project, thus getting your program. Depending on what you want,
   you'll run one of the commands below:
    - If you want both the API and scraper functionality, you can just run the following command:
      ```
      cargo build --release
      ```
    - If you _only_ want the API functionality, run the command:
      ```
      cargo build --no-default-features --features api --release
      ```
    - If you _only_ want the scraper functionality, run the command: 
      ```
      cargo build --no-default-features --features scraper --release
      ```

5. You should find the executable in the `/target/release` directory. Run the executable using the following command:
   ```
   ./webreg_scraper path_to_config_file
   ```
   where `path_to_config_file` should be replaced with the configuration file's name (full path). You may need to run
   `chmod u+x webreg_scraper` first so you can execute the program!

</details>

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


## License
Everything in this repository is licensed under the MIT license.