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

## Authentication Modes
Prior to March 26, 2024, this script supported either Push or SMS mode. Now, because of the new 
[Duo Universal Prompt](https://blink.ucsd.edu/technology/security/services/two-step-login/universal-prompt.html),
only Push is supported. 

This script only supports Duo Push: when the script is starting up, the script will initially authenticate you using Duo Push.

Keep in mind that you'll need to restart the login script setup process every 6-7 days to ensure you can still keep 
yourself logged in. 

~~Also, Ruby is a clown~~

## Requirements
In order to ensure that you _can_ use this script, ensure that the following technical and non-technical requirements
are satisfied.

### Non-Technical
- You must have a UCSD account that gives you access to WebReg.
- Your UCSD account must be configured so that a Duo push is automatically sent when needed (i.e., set Duo Push as the 
**default authentication method**). See [this UCSD ITS article](https://support.ucsd.edu/its?id=kb_article_view&sys_kb_id=f91e1f55873259d8947a0fa8cebb352e&sysparm_article=KB0030238) for more information.

> [!NOTE] 
> Starting March 26, 2024, with the introduction of the [Duo Universal Prompt](https://blink.ucsd.edu/technology/security/services/two-step-login/universal-prompt.html),
> Duo Push should automatically be done regardless of what you've chosen above.

### Technical
- You'll need to have [Node.js](https://nodejs.org/en/) installed. The long term support (LTS) version will do.
- If you're using Ubuntu, you'll also need to ensure that the following system dependencies are installed.
    ```bash
    sudo apt-get update
    sudo apt install libgtk-3-dev libnotify-dev libgconf-2-4 libnss3 libxss1 libasound2
    ```
    This was taken from [here](https://github.com/puppeteer/puppeteer/blob/main/docs/troubleshooting.md#running-puppeteer-on-wsl-windows-subsystem-for-linux).

## Setup
To actually run this script, follow the directions below.

1. Install TypeScript globally:
    ```bash
    npm i -g typescript 
    ```
   As the script is written in TypeScript, you need to install TypeScript to "compile" the script.

2. Install the project dependencies that this script needs using the command:
    ```bash
    npm i
    ```

3. Run the following command to compile the script:
    ```bash
    npm run compile
    ```
   This is an alias for the command `tsc -p .`, which is defined in the `package.json` file.

4. At this point, you should see an `out` folder. In the `out` folder, you should have an `index.js` file. You must provide your credentials via environment variables to run the script securely. You can run the command inline like so:
    ```bash
    WEBREG_USERNAME="your_username" WEBREG_PASSWORD="your_password" node out/index.js --port <port>
    ```
    where
    - `port` is the port where this script should be "visible" to the scraper. Usually, I put a number like `3001` or `4000`.
    
    Alternatively, export the variables to your shell first:
    ```bash
    export WEBREG_USERNAME="your_username"
    export WEBREG_PASSWORD="your_password"
    node out/index.js --port 3001
    ```

    When running the command for the first time, follow the directions that are presented in the console. After initial setup is complete, the script is ready to serve future login requests.

> **Warning:**
> If you use `push` mode, you'll need to repeat this process every 6-7 days to ensure your scraper runs uninterrupted.

## Environment Variables
The script relies on the following environment variables for configuration instead of a hardcoded JSON file:

- `WEBREG_USERNAME` (**Required**): Your UCSD Active Directory username.
- `WEBREG_PASSWORD` (**Required**): Your UCSD Active Directory password.
- `LOGIN_TYPE` (*Optional*): The login process you want to use. Defaults to `push`. This should currently only be `push`.
- `AUTOMATIC_PUSH_ENABLED` (*Optional*): Whether your account is configured to automatically send a Duo Push on login. Accepts `"true"` or `"false"`. 

> [!NOTE]
> `AUTOMATIC_PUSH_ENABLED` is largely a legacy setting and will be deprecated in a later version due to Universal Prompt changes.