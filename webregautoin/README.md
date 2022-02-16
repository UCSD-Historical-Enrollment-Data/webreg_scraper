# webregautoin
A basic API server designed to automatically get cookies for a valid WebReg session when needed. 

## Programming Language
This mini-project uses the latest stable version of TypeScript and Node.js.

## Why & How?
WebReg generally goes into maintenance mode around 4:15AM PT. When WebReg goes into maintenance mode, all active and valid sessions become invalidated. Sometimes, I need to keep myself logged into WebReg 24/7. So, I make use of this little API server to do the job for me.

The API server uses [a headless Chrome browser](https://github.com/puppeteer/puppeteer) to log into WebReg and get the new cookies. In the initial setup process, the headless Chrome browser will essentially log you in with the given credentials and then automatically select the `Remember me for 7 days` checkbox when performing Duo authentication. That way, you don't need to worry about having to authenticate via Duo for the next 7 days.

## License
This mini-project uses the same license as the main project. 