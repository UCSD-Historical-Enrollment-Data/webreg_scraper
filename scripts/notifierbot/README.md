# notifierbot
A basic program that notifies the user via Discord when the scraper is not working.

## Programming Language
This mini-project uses the latest stable version of TypeScript and Node.js.

## Why & How?
Sometimes, either my [login script](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin) or the scraper itself fails for some reason. I wanted a way to be notified when my scraper is unable to scrape data so I can remedy the issue as soon as possible. Since I'm on Discord very often, I decided that I would use Discord's webhook feature to notify me when something goes wrong.

All this script does is checks the scraper's status every 1 minute. Specifically, it just makes a request to a random class and sees if the scraper returns a valid response or an error. If it returns an error, the scraper will notify the user. It should be noted that the script will _not_ check the status between 4:00 AM and 5:00 AM Pacific time since this is usually when WebReg itself is offline.

## License
This mini-project uses the same license as the main project. 