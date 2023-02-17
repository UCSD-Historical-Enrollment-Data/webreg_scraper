// The idea is to have a headless browser running in the background 24/7. This headless browser
// will already be logged into WebReg (in particular, for Duo, we checked the "Remember me for
// 7 days" box). So, whenever our cookies expire (which seems to occur around 4-4:30am), we can
// just make a request to our API here. Our API will then automatically log into WebReg through
// the headless browser and then return the cookies which can then be used by the tracker
// application. 

import * as fs from "fs";
import * as path from "path";
import * as puppeteer from "puppeteer";
import * as http from "http";

const DEBUG_MODE: boolean = false;

// The term to select. A term MUST be selected in order for the resulting cookies to be valid.
// Use inspect elements on the dropdown box to get the option value. Specifically, TERM should 
// be the value given by 
// <option value="THIS">Some Quarter</option>
//                ----
const ALL_TERMS: readonly string[] = [
    "5260:::WI23",
    "5270:::SP23"
];

const NUM_ATTEMPTS_BEFORE_EXIT: number = 6;

function printHelpMessage(): void {
    console.error("Usage: node index.js <term> <port>");
    console.error(`\tWhere <term> can be one of: ${ALL_TERMS.map(x => x.split(":::")[1]).join(", ")}.`);
    console.error("\tWhere <port> is an integer.");
    console.error("Example: node index.js SP22 3000");
}

// Read command line arguments, which should be in the form
//           node index.js <term> <port>
//           e.g. node index.js S122 3000
const args = process.argv.slice(2);
if (args.length < 2) {
    printHelpMessage();
    process.exit(1);
}

const tempTerm = args[0].toUpperCase().trim();
const termToUse = ALL_TERMS.find(x => x.endsWith(tempTerm));
const port = Number.parseInt(args[1], 10);
if (!termToUse || Number.isNaN(port)) {
    printHelpMessage();
    process.exit(1);
}


// Constants & other important variables
let BROWSER: puppeteer.Browser | null = null;
// When this instance started, represented as a unix timestamp
let START_SESSION: number = 0;
// Whenever a *successful* call to this API occurs (for login cookies),
// add the unix timestamp corresponding to the time of the request
// here. 
let SUCCESS_CALL_HISTORY: number[] = [];
// Config information (should contain webreg credentials)
const CONFIG: IConfiguration = JSON.parse(
    fs.readFileSync(path.join(__dirname, "..", "credentials.json")
    ).toString());
const WEBREG_URL: string = "https://act.ucsd.edu/webreg2/start";

interface IConfiguration {
    username: string;
    password: string;
}

/**
 * Logs a message.
 * @param msg The message to log.
 */
function log(msg: string): void {
    const time = new Intl.DateTimeFormat([], {
        timeZone: "America/Los_Angeles",
        year: "numeric",
        month: "numeric",
        day: "numeric",
        hour: "numeric",
        minute: "numeric",
        second: "numeric",
    }).format(new Date());
    console.info(`[${time}] [${termToUse}] ${msg}`);
}

/**
 * Waits a certain number of milliseconds before continuing execution.
 * @param ms The number of milliseconds to wait.
 */
function waitFor(ms: number): Promise<void> {
    return new Promise(async r => {
        setTimeout(() => {
            r();
        }, ms);
    });
}

/**
 * Gets new WebReg cookies.
 * @returns The cookies.
 */
async function getCookies(): Promise<string> {
    log(`GetCookies function called.`)
    if (!BROWSER) {
        log("Launching browser for first-time setup.");
        BROWSER = await puppeteer.launch({
            args: ['--no-sandbox', '--disable-setuid-sandbox'],
            // If debug mode is on, turn OFF headless mode
            headless: !DEBUG_MODE 
        });
    }

    let numFailedAttempts = 0;
    while (true) {
        // Close any unnecessary pages. 
        let pages = await BROWSER.pages();
        while (pages.length > 1) {
            await pages.at(-1)!.close();
            pages = await BROWSER.pages();
        }

        const page = await BROWSER.newPage();
        try {
            log("Opened new page. Attempting to connect to WebReg site.")
            const resp = await page.goto(WEBREG_URL);
            log(`Reached ${resp.url()} with status code ${resp.status()}.`);
            if (resp.status() < 200 || resp.status() >= 300) {
                throw new Error("Non-OK Status Code Returned.");
            }
        }
        catch (e) {
            // Timed out probably, or failed to get page for some reason.
            log(`An error occurred when trying to reach WebReg. See error stack trace below.`);
            console.info(e);
            console.info();
            return "";
        }

        await waitFor(3000);
        const content = await page.content();
        // This assumes that the credentials are valid.
        if (content.includes("Signing on Using:") && content.includes("TritonLink user name")) {
            log("Attempting to sign in to TritonLink.");
            // https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
            await page.type('#ssousername', CONFIG.username);
            await page.type('#ssopassword', CONFIG.password);
            await page.click('button[type="submit"]');
        }

        // Wait for either Duo 2FA frame (if we need 2FA) or "Go" button (if no 2FA needed) to show up
        log("Waiting for Duo 2FA frame or 'Go' button to show up.");


        let loggedIn = false;
        const r = await Promise.race([
            // Either wait for the 'Go' button to show up, which implies that we
            // have an authenticated session, **OR** wait for the Duo frame
            // to show up. 
            //
            // If an error occurred, it means the 'Go' button could not be found
            // after 30 seconds. This implies that the Duo frame could not be
            // found since *if* the Duo frame did show up, then the error would
            // have never occurred. 

            // Here, we wait for the 'Go' button (to load WebReg for a term) to
            // show up.
            (async () => {
                try {
                    await page.waitForSelector("#startpage-button-go", { visible: true, timeout: 30 * 1000 });
                } catch (_) {
                    // conveniently ignore the error
                    return 2;
                }
                return 0;
            })(),
            // Here, we *repeatedly* check to see if the Duo 2FA frame is visible AND some components of
            // the frame (in our case, the "Rememebr Me" checkbox) are visible.
            (async () => {
                const interval = await new Promise<NodeJS.Timeout>(r => {
                    const internalInterval = setInterval(async () => {
                        try {
                            // If we're logged in, then we can stop the interval.
                            if (loggedIn) {
                                r(internalInterval);
                                return;
                            }

                            const possDuoFrame = await page.$("iframe[id='duo_iframe']");
                            if (!possDuoFrame) {
                                return;
                            }

                            const duoFrame = await possDuoFrame.contentFrame();
                            if (!duoFrame) {
                                return;
                            }

                            if (!(await duoFrame.$("#remember_me_label_text"))) {
                                return;
                            }

                            r(internalInterval);
                        }
                        catch (e) {
                            // Conveniently ignore the error
                        }
                    }, 500);
                });

                clearInterval(interval);
                return 1;
            })()
        ]);

        // If we hit this, then we just try again.
        if (r === 2) {
            // If too many failed attempts, then notify the caller.
            // After all, we don't want to make too many Duo pushes and get 
            // the AD account blocked by ITS :)
            if (numFailedAttempts >= NUM_ATTEMPTS_BEFORE_EXIT) {
                log("Unable to authenticate due to too many attempts reached, giving up.")
                return "ERROR UNABLE TO AUTHENTICATE.";
            }

            // Not sure why we have this here
            // loggedIn = true;
            numFailedAttempts++;
            log(`Unable to find a 'Go' button or Duo 2FA frame. Retrying (${numFailedAttempts}/${NUM_ATTEMPTS_BEFORE_EXIT}).`);
            continue;
        }

        log(
            r === 0
                ? "'Go' button found. No 2FA needed."
                : "Duo 2FA frame found. Ignore the initial 2FA request; i.e., do not"
                + " accept the 2FA request until you are told to do so."
        );

        if (r === 0) {
            loggedIn = true;
        }

        // Wait an additional 4 seconds to make sure everything loads up.
        await waitFor(4 * 1000);

        // No go button means we need to log in.
        // We could just check if (r === 1) though
        if (!(await page.$("#startpage-button-go"))) {
            log("Beginning Duo 2FA process. Do not accept yet.");
            // Need to find a duo iframe so we can actually authenticate 
            const possDuoFrame = await page.$("iframe[id='duo_iframe']");
            if (!possDuoFrame) {
                log("No possible Duo frame found. Returning empty string.");
                console.info();
                return "";
            }

            const duoFrame = await possDuoFrame.contentFrame();
            if (!duoFrame) {
                log("Duo frame not attached. Returning empty string.");
                console.info();
                return "";
            }

            // it's possible that we might need to cancel our existing authentication request,
            // especially if we have duo push automatically send upon logging in
            await waitFor(1000);
            const cancelButton = await duoFrame.$(".btn-cancel");
            if (cancelButton) {
                await cancelButton.click();
                log("Clicked the CANCEL button to cancel initial 2FA request. Do not respond to 2FA request.");
            }

            await waitFor(1000);
            // Remember me for 7 days
            await duoFrame.click('#remember_me_label_text');
            log("Checked the 'Remember me for 7 days' box.");
            await waitFor(1000);
            // Send me a push 
            await duoFrame.click('#auth_methods > fieldset > div.row-label.push-label > button');
            log("A Duo push was sent. Please respond to the new 2FA request.");
        }

        try {
            await Promise.all([
                page.waitForSelector("#startpage-select-term", { visible: true }),
                page.waitForSelector('#startpage-button-go', { visible: true })
            ]);
        }
        catch (e) {
            // If this hits, then somehow the Go button (for loading WebReg with that term) 
            // didn't load at all. This is rare, although it does happen from time to time 
            // for reasons I have yet to understand.
            //
            // Note that I used a try/catch in Promise.all instead of Promise.allSettled
            // because waitForSelector apparently throws the error instead of rejecting?
            // Not sure if there's a way to handle that without try/catch
            log("Could not find select term dropdown or Go button.");
            console.info(e);
            console.info();
            return "";
        }

        log("Logged into WebReg successfully.");

        await page.select("#startpage-select-term", termToUse!);
        const term = termToUse!.split(":::").at(-1) ?? "";
        // Get cookies ready to load.
        await page.click('#startpage-button-go');
        const cookies = await page.cookies(`https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-get-schednames?termcode=${term}`);
        log(`Extracted cookies for term '${term}' and responding back with them.\n`);

        if (START_SESSION === 0) {
            START_SESSION = Date.now();
        }
        else {
            SUCCESS_CALL_HISTORY.push(Date.now());
        }

        return cookies.map(x => `${x.name}=${x.value}`).join("; ");
    }
}

// Very basic server. 
const server = http.createServer(async (req, res) => {
    if (req.method !== "GET") {
        res.end(
            JSON.stringify({
                error: http.STATUS_CODES[405]
            })
        );

        return;
    }

    if (req.url === "/cookie") {
        res.end(
            JSON.stringify({
                cookie: await getCookies()
            })
        );
    }
    else if (req.url === "/history") {
        res.end(
            JSON.stringify(SUCCESS_CALL_HISTORY)
        );
    }
    else if (req.url === "/start") {
        res.end(
            JSON.stringify(START_SESSION)
        );
    }
    else {
        res.end(
            JSON.stringify({
                error: http.STATUS_CODES[404]
            })
        );
    }
});

server.listen(port, () => {
    log(`Server listening on port ${port}`);
});

process.on('SIGTERM', shutDown);
process.on('SIGINT', shutDown);

async function shutDown(): Promise<void> {
    log("Shutting down server & closing browser.");
    BROWSER?.close();
    server.close();
}

// Warm-up call.
getCookies().then();