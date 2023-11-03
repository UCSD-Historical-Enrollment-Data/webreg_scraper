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
As of recently, this script supports either Push or SMS mode. The modes only really matter at the beginning (i.e., when
you start the script).
- Push mode essentially means that, when the script is starting up, the script will initially authenticate you using 
  Duo Push.
- SMS mode means that, when the script is starting up, the script will initially use the SMS code that best fits the 
  hint that is given (i.e., it will try to find the code you defined in the configuration file that satisfies the hint
  "Your next SMS Passcode starts with XXX"). 

Push mode is probably the easiest to use short-term, but you'll need to restart the login script setup process every
6-7 days to ensure you can still keep yourself logged in. SMS mode is somewhat easy, and allows you to remember your
session for up to 70 days (10 codes times 7 days per code = 70 days). However, you probably won't be able to use SMS
mode _outside_ of this application.

As a warning, SMS mode is not guaranteed to work as expected. Duo Push remains the most stable login process. 

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

1. Two sample configuration files have been provided for you: `credentials.sample_push.json` and `credentials.sample_sms.json`.
    1. Rename this file to `credentials.json`.
    2. Open the file and fill in your UC San Diego Active Directory username and password.
    3. Modify any other relevant settings (see the next section on the configuration file for more on this).
    4. Save your changes.

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
> If you use `push` mode, you'll need to repeat this process every 6-7 days to ensure your scraper runs uninterrupted.
> 
> If you use `sms` mode, you'll need to repeat this process every 70 days or so, but you must not use SMS mode outside
> of this program. In other words, if you use `sms` mode for this application, do not use Duo SMS outside of this app.

## Configuration File Layout
There are two sample configuration files you can use; each of them correspond to the type of login process you can use
for this login script.

Both configuration layouts will feature the same keys:
- `webreg.username` (`string`): Your UCSD Active Directory username.
- `webreg.password` (`string`): Your UCSD Active Directory password.
- `settings.loginType` (`sms` or `push`): The login process you want to use. This can only be `sms` or `push`.
- `settings.automaticPushEnabled` (`boolean`): Whether your account is configured to automatically sends a Duo Push on 
  login. If this value is `true`, then the login script will cancel the automatic push when setting itself up. 

### Duo Push
```json
{
    "webreg": {
        "username": "",
        "password": ""
    },
    "settings": {
        "loginType": "push",
        "automaticPushEnabled": true
    }
}
```

### Duo SMS
```json
{
    "webreg": {
        "username": "",
        "password": ""
    },
    "settings": {
        "loginType": "push",
        "automaticPushEnabled": true,
        "smsTokens": [
            "your",
            "sms",
            "tokens",
            "as",
            "strings"
        ]
    }
}
```

Additionally, Duo SMS configuration files have a third settings property:
- `settings.smsTokens` (`string[]`): a list of SMS tokens that Duo sent you. To obtain these tokens, log into your UCSD
  account. When you reach the Duo 2FA screen, select "Enter a Passcode" and then click on "Text me new codes." You should
  receive the tokens via text. When you do, just put the tokens into `settings.smsTokens`, ensuring that each element is
  of type _string_ (**not** an integer).