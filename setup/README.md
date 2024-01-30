# Setup Environment
You can use the `setup.sh` script to install all necessary files and dependencies needed to run this project. At this time, the script has been tested and works on Ubuntu OS version 23.10. 

> [!WARNING]
> Other Linux distributions have not been tested. You may need to adjust the script to work as expected.

I recommend creating a [DigitalOcean droplet](https://www.digitalocean.com/); the cheapest plan will suffice, and students with the GitHub student pack are eligible for [$200 in DigitalOcean credits for 1 year](https://education.github.com/pack/offers). 

---

> [!NOTE]
> Before beginning, I recommend that you run this script on a non-root user with sudo access. 
>
> If you plan on running this script using `root`, and then later plan on managing the scraper from a non-root user, you'll need to complete steps 3-5 again for that user.

To start, copy both `setup.sh` and `nginx.conf` to the directory where you want all necessary project files to be stored at (e.g., your home directory, `~`). Then, update your system if needed (e.g., using `apt-get update`). Afterwards, run `sudo setup.sh`. This script will
1. Set the timezone to Pacific Time
2. Install all dependencies needed for puppeteer to work
3. Install `nvm` and the LTS version of `node.js`
    - Install `pm2` and `typescript` globally on the current account
4. Download the latest version of the WebReg scraper with the `authmanager` executable
5. Clone the repository, and extract and compile the `notifier` and `webregautoin` scripts.
6. Install `nginx` and replace the default configuration file with the one provided here. 

In other words, this setup script will get your environment ready to run the scraper and the login script, and make the scraper's API available to anyone through `nginx`. You do not need to install anything else to make this script work.