# Setup Environment
If you have a new _Ubuntu_ environment, you can use the `setup.sh` script to install all necessary files and dependencies needed to run this project. Other Linux distributions have not been tested.

I recommend creating a [DigitalOcean droplet](https://www.digitalocean.com/); the cheapest plan will suffice, and students with the GitHub student pack are eligible for [$200 in DigitalOcean credits for 1 year](https://education.github.com/pack/offers). 

To start, copy both `setup.sh` and `nginx.conf` to the directory where you want all necessary project files to be stored at (e.g., your home directory, `~`). Then, run `sudo setup.sh`. This script will
- Set the timezone to Pacific Time
- Install all dependencies needed for puppeteer to work
- Install `nvm` and the LTS version of `node.js`
    - Install `pm2` and `typescript` globally
- Download the latest version of the WebReg scraper with the `authmanager` executable
- Clone the repository, and extract and compile the `notifier` and `webregautoin` scripts.
- Install `nginx` and replace the default configuration file with the one provided here. 

In other words, this setup script will get your environment ready to run the scraper and the login script, and make the scraper's API available to anyone through `nginx`. You do not need to install anything else to make this script work.