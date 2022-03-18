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

// The term to select. A term MUST be selected in order for the resulting cookies to be valid.
// Use inspect elements on the dropdown box to get the option value. Specifically, TERM should 
// be the value given by 
// <option value="THIS">Some Quarter</option>
//                ----
const TERM: string = "5200:::SP22";

// Constants & other important variables
let BROWSER: puppeteer.Browser | null = null;
const CONFIG: IConfiguration = JSON.parse(
    fs.readFileSync(path.join(__dirname, "..", "credentials.json")
    ).toString());
const PORT: number = 3000;
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
    console.info(`[${time}] ${msg}`);
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
    log("GetCookies function called.")
    if (!BROWSER) {
        log("Launching browser for first-time setup.");
        BROWSER = await puppeteer.launch({
            args: ['--no-sandbox', '--disable-setuid-sandbox']
        });
    }

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
        log(`An error occurred. Returning empty string. See error stack trace below.`);
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
        // have an authenticated session.
        (async () => {
            await page.waitForSelector("#startpage-button-go", { visible: true, timeout: 30 * 1000 });
            return 0;
        })(),
        // Or, we *repeatedly* check to see if the Duo 2FA frame is visible AND some components of
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
                }, 1000);
            });

            clearInterval(interval);
            return 1;
        })()
    ]);

    log(
        r === 0
            ? "'Go' button found. No 2FA needed."
            : "Duo 2FA frame found. Ignore the initial 2FA request; i.e. do not"
            + " accept the 2FA request until you are told to do so."
    );

    if (r === 0) {
        loggedIn = true;
    }

    // Wait an additional 4 seconds to make sure everything loads up.
    await waitFor(4 * 1000);

    // No go button means we need to log in.
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

    await Promise.all([
        page.waitForSelector("#startpage-select-term", { visible: true }),
        page.waitForSelector('#startpage-button-go', { visible: true })
    ]);
    log("Logged into WebReg successfully.");

    await page.select("#startpage-select-term", TERM);
    const term = TERM.split(":::").at(-1) ?? "";
    // Get cookies ready to load.
    await page.click('#startpage-button-go');
    const cookies = await page.cookies(`https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-get-schednames?termcode=${term}`);
    log(`Extracted cookies for term '${term}' and responding back with them.\n`);
    return cookies.map(x => `${x.name}=${x.value}`).join("; ");
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

        return;
    }

    res.end(
        JSON.stringify({
            error: http.STATUS_CODES[404]
        })
    );
});

server.listen(PORT, () => {
    log(`Server listening on port ${PORT}`);
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