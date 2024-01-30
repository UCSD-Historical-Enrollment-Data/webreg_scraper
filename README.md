<p align="center">
  <img src="https://github.com/ewang2002/webreg_scraper/blob/master/assets/project_banner.png?raw=true"  alt="Project Banner"/>
</p>

<p align="center">
  <a href="https://github.com/ewang2002/webweg">webweg</a> |
  <b>webreg_scraper</b> |
  <a href="https://github.com/ewang2002/UCSDHistEnrollData">UCSDHistEnrollmentData</a>
</p>

A program designed to both scrape UCSD's WebReg for enrollment data and make the WebReg API available to other applications.

## Crates
This project is broken up into two binary crates, defined by a workspace. To see more information about them, just click
on the crate name.

| Binary Crate | Information |
| ------------ |-------------|
| [`webreg`](https://github.com/ewang2002/webreg_scraper/tree/master/crates/webreg) | This is the actual scraper _and_ API application. |
| [`authmanager`](https://github.com/ewang2002/webreg_scraper/tree/master/crates/authmanager) | A simple authentication manager for the API. |

`webreg` is the main binary in this project. Therefore, the project version is based on `webreg`'s version.


## Scripts
This repository contains two scripts, one of which is required for the scraper to work properly. To see more information 
about them, just click on the script name.

| Script Name | Information |
| ----------- |-------------|
| [`webregautoin`](https://github.com/ewang2002/webreg_scraper/tree/master/scripts/webregautoin) | A basic web server designed to automatically log the scraper into WebReg. **This is required.** |
| [`notifierbot`](https://github.com/ewang2002/webreg_scraper/tree/master/scripts/notifierbot) | A simple script that uses Discord webhooks to notify you if the scraper is not working. |

This program _requires_ the `webregautoin` helper program.

## Setup
To run this project, feel free to explore the individual scripts or crates above; setup guides for each are provided.

If you want to get an Ubuntu environment ready with all the necessary files needed to run this project, you can run the setup script in the [`setup`](https://github.com/ewang2002/webreg_scraper/tree/master/setup) folder. More information will be provided there.

## License
Everything in this repository is licensed under the MIT license.