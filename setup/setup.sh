#!/bin/bash

if [ "$(id -u)" -ne 0 ]; then 
	echo "Please run w/ sudo." >&2; 
	exit 1; 
fi

apt-get update

# Set the timezone to Pacific time
timedatectl set-timezone America/Los_Angeles

# Install puppeteer dependencies
# See https://github.com/puppeteer/puppeteer/blob/main/docs/troubleshooting.md#running-puppeteer-on-wsl-windows-subsystem-for-linux
apt install -y libgtk-3-dev libnotify-dev libnss3 libxss1 libasound2

# Install jq for JSON parsing (we need this when getting the latest release tag for the scraper)
apt install -y jq

# Install nvm and the LTS version of node.js
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.5/install.sh
source ~/.bash_profile
nvm install --lts
source ~/.bash_profile

# Install pm2 and typescript
npm install -g pm2
npm install -g typescript

# Get latest scraper tag
tag=$(curl --silent "https://api.github.com/repos/ewang2002/webreg_scraper/releases" | jq -r "first | .tag_name")

# Setup webreg_scraper
mkdir scraper
cd scraper
wget https://github.com/ewang2002/webreg_scraper/releases/download/$tag/authmanager-x86_64-unknown-linux-gnu.tar.gz
tar -xvzf authmanager-x86_64-unknown-linux-gnu.tar.gz
wget https://github.com/ewang2002/webreg_scraper/releases/download/$tag/webreg-x86_64-unknown-linux-gnu-auth.tar.gz
tar -xvzf webreg-x86_64-unknown-linux-gnu-auth.tar.gz
rm *.tar.gz
cd ..

# Setup the login & notifier scripts
git clone https://github.com/ewang2002/webreg_scraper
mv webreg_scraper/scripts/notifierbot notifier
mv webreg_scraper/scripts/webregautoin login
rm -rf webreg_scraper

# Setup the notifier
cd notifier
npm i
npm run compile
cd ..

# Setup the login script
cd login
npm i
npm run compile
cd ..

# Install nginx
sudo apt install -y nginx
rm -f /etc/nginx/nginx.conf
mv nginx.conf /etc/nginx

# All done.
echo "All done. Make sure to configure the notifier bot, login script, and the scraper."
exit 0
