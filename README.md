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

The recommended way to run the scraper is via Docker, as it abstracts away all
the dependency management and setup. You can find the Docker setup guide in the 
[`docker`](./docker/) directory.

Feel free to explore the individual scripts or crates above. 

If you want to run the scraper without Docker, you can follow the instructions in 
the respective README files for the scripts and crates.  


> [!NOTE]
> You can also run the setup script in the [`setup`](https://github.com/ewang2002/webreg_scraper/tree/master/setup) 
> folder. More information will be provided there. Do note that some of the packages
> that the script installs are deprecated and may cause issues. The script is tested 
> to work on Ubuntu 23.04.

## License
Everything in this repository is licensed under the MIT license.