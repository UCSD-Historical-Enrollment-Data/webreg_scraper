# webregautoin
A basic web server designed to automatically get cookies for a valid WebReg session when needed.

## Why & How?
WebReg generally goes into maintenance mode around 4:15AM PT. When WebReg goes into maintenance mode, all active
and valid sessions become invalidated. The scraper requires access to WebReg 24/7, so I make use of this little web
server to ensure that the scraper remains logged in.

The API server uses [a headless Chrome browser](https://github.com/puppeteer/puppeteer) to log into WebReg and get 
the new cookies. In the initial setup process, the headless Chrome browser will essentially log you in with the given 
credentials and then automatically select the `Remember me for 7 days` checkbox when performing Duo authentication. 
That way, you don't need to worry about having to authenticate via Duo for the next 7 days.

## Requirements
In order to ensure that you _can_ use this script, ensure that the following technical and non-technical requirements
are satisfied.

### Non-Technical
- You must have a UCSD account that gives you access to WebReg.
- Your UCSD account must be configured so that a Duo push is automatically sent when needed (i.e., set Duo Push as the 
**default authentication method**). See [this UCSD ITS article](https://support.ucsd.edu/its?id=kb_article_view&sys_kb_id=f91e1f55873259d8947a0fa8cebb352e&sysparm_article=KB0030238) for more information.

### Technical
- You'll need to have [Node.js](https://nodejs.org/en/) installed. The long term support (LTS) version will do.
- If you're using Ubuntu, you'll also need to ensure that the following system dependencies are installed.
    ```
    sudo apt-get update
    sudo apt install libgtk-3-dev libnotify-dev libgconf-2-4 libnss3 libxss1 libasound2
    ```
    This was taken from [here](https://github.com/puppeteer/puppeteer/blob/main/docs/troubleshooting.md#running-puppeteer-on-wsl-windows-subsystem-for-linux).

## Setup
To actually run this script, follow the directions below.

1. A sample configuration file has been provided for you; this file is called `credentials.example.json`.
    1. Rename this file to `credentials.json`.
    2. Open the file and fill in your UC San Diego Active Directory username and password.
    3. Save your changes.

2. Next, install TypeScript globally:
    ```
    npm i -g typescript 
    ```
   As the script is written in TypeScript, you need to install TypeScript to "compile" the script.

3. Install the project dependencies that this script needs using the command:
    ```
    npm i
    ```

4. Run the following command to compile the script:
    ```
    npm run compile
    ```
   This is an alias for the command `tsc -p .`, which is defined in the `package.json` file.

5. At this point, you should see an `out` folder. In the `out` folder, you should have an `index.js` file. You can
   run the command like so:
    ```
    node index.js --port <port>
    ```
    where
    - `port` is the port where this script should be "visible" to the scraper. Usually, I put a number like `3001` or
      `4000`.

    When running the command for the first time, follow the directions that are presented. After initial setup is 
    complete, then the script is ready to serve future login requests.

> **Warning:**
> You'll need to repeat this process every 6-7 days to ensure your scraper runs uninterrupted.