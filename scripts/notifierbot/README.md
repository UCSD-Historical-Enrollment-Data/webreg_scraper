# notifierbot
A basic program that notifies the user via Discord when the scraper is not working.

## Why & How?
Sometimes, either my [login script](https://github.com/ewang2002/webreg_scraper/tree/master/webregautoin) or the scraper itself fails for some reason. 
I wanted a way to be notified when my scraper is unable to scrape data so that I can remedy the issue as soon as possible. 
Since I'm on Discord very often, I decided that I would use Discord's webhook feature to notify me when something goes wrong.

All this script does is checks the scraper's status every few minutes. Specifically, it just makes a request to a random 
class and sees if the scraper returns a valid response or an error. If it returns an error, the scraper will notify 
the user. It should be noted that the script will _not_ check the status between 4:00 AM and 5:00 AM Pacific time 
since this is usually when WebReg itself is offline.

## Requirements
In order to ensure that you _can_ use this script, ensure that the following technical and non-technical requirements
are satisfied.

### Non-Technical
- Create a webhook in the Discord channel where you wish to receive notifications. For more information, please see
  [this resource](https://support.discord.com/hc/en-us/articles/228383668-Intro-to-Webhooks). Make sure you have the
  webhook URL ready.
  - You can create multiple webhooks if you want.

### Technical
- You'll need to have [Node.js](https://nodejs.org/en/) installed. The long term support (LTS) version will do.
- You'll need to make sure the login script and the scraper is running before you start this script up.

## Setup
To actually run this script, follow the directions below.

1. A sample configuration file has been provided for you; this file is called `config.example.json`. Rename this file
   to `config.json`.

   You'll notice that the file looks something like
   ```json
   {
      "terms": [],
      "webhookUrls": [
         {"url": "", "peopleToPing": []},
         {"url": "", "peopleToPing": []}
      ],
      "apiKey": ""
   }
   ``` 

    In terms of what everything here means:
    - `terms` is an array of terms that the scraper is also collecting data for. More technically, `terms` should be a 
        _subset_ of what the scraper is collecting data for. For example, if the scraper is collecting enrollment data
        for `FA23` and `S223`, then your `terms` array here should have `FA23` and/or `S223`.
    - `webhookUrls` is an array of objects, where each object has two properties:
      - `url` is the webhook URL that was mentioned earlier.
      - `peopleToPing` is an array of Discord IDs that should be pinged when something goes wrong.
    - `apiKey` is a string representing the authentication key to make API requests. This is _optional_ if and only if
       the `auth` feature for `webreg` is not enabled (i.e., no authentication required). So, if you don't have
       authentication, you can leave this value blank or remove this key, value pair.

    In the example above, there are two objects in the array associated with `webhookUrls`; in reality, you can have as
    many (or as little) webhooks as you want. An example configuration file might look like:
    ```json
    {
        "terms": ["FA23", "S223"],
        "webhookUrls": [
            {"url": "https://ptb.discord.com/api/webhooks/...", "peopleToPing": ["348573489572485545"]},
            {"url": "https://discord.com/api/webhooks/...", "peopleToPing": ["587345345345636234", "128361273162731234"]},
            {"url": "https://discord.com/api/webhooks/...", "peopleToPing": ["123123123213123123"]}
        ]
    }
    ```
   
    It is assumed that the WebReg server is running on `localhost:3000`. If this is _not_ the case, you'll need to
    manually make changes to the code.

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
    node index.js
    ```
   
    Keep in mind that this script should be executed on the same server where the actual WebReg binary is being executed.