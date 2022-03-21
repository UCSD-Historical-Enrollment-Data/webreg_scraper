# ucsd_webreg_rs
A program designed to interact with UCSD's WebReg.

## Programming Language
The main project (API wrapper) uses the latest version of [Rust](https://www.rust-lang.org/).

<details>
<summary>More Information</summary>
<br> 

The reason why I chose Rust instead of, say, Python or C#, is because I wanted to learn more about Rust. Plus, I've been meaning to work on a project with Rust.

There is additionally another project, creatively namd `webregautoin`, which uses Node's [HTTP](https://nodejs.org/api/http.html) library to create a local API server which the wrapper can use. In particular, this local API has one sole purpose: when new cookies are needed to log into WebReg, the wrapper can make a request to the local API. The local API will then use [a headless Chrome browser](https://github.com/puppeteer/puppeteer) to log into WebReg and get the new cookies. Note that you'll need to log into WebReg beforehand so you can select the `Remember me for 7 days` checkbox for the Duo 2FA (this will automatically be done when an initial request is made).

</details>


## Features
This program can do the following.
- Create conflict-free schedules (not exactly efficiently, but it works).
- Tracks enrollment counts and saves this information to a CSV file.
- Exports all available sections into a CSV file.
- Host a small web server (using [Rocket](https://rocket.rs/)) to make it easy for other applications to use the data.

## Webweg
Note that this program makes use of the [webweg](https://github.com/ewang2002/webweg) crate, which I developed.

## Future Plans
Whenever I have free time and I'm not busy with other projects, I definitely plan on coming back to this project and:
- Add more stuff to it (while also learning Rust)
- Clean up a lot of the code.
- And more!

## License
Everything in this repository is licensed under the MIT license.