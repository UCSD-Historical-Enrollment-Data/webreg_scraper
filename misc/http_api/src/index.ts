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

let BROWSER: puppeteer.Browser | null = null;
const CONFIG: IConfiguration = JSON.parse(
    fs.readFileSync(path.join(__dirname, "..", "credentials.json")
    ).toString());
const PORT: number = 3000;

interface IConfiguration {
    username: string;
    password: string;
}

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
    if (!BROWSER) {
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
    await page.goto("https://act.ucsd.edu/webreg2/start");
    await waitFor(3000);
    const content = await page.content();
    if (content.includes("Signing on Using:") && content.includes("TritonLink user name")) {
        console.info("Attempting to log in.");
        // https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
        await page.type('#ssousername', CONFIG.username);
        await page.type('#ssopassword', CONFIG.password);
        await page.click('button[type="submit"]');
        await waitFor(5 * 1000);
    }

    // No go button means we need to log in.
    if (!(await page.$('#startpage-button-go'))) {
        console.info("Attempting to authenticate with Duo.");
        // Need to find a duo iframe so we can actually authenticate 
        const possDuoFrame = await page.$("iframe[id='duo_iframe']");
        if (!possDuoFrame) {
            console.error("no possible duo frame found.");
            return "";
        }

        const duoFrame = await possDuoFrame.contentFrame();
        if (!duoFrame) {
            console.error("no duo frame attached.");
            return "";
        }

        // it's possible that we might need to cancel our existing authentication request,
        // especially if we have duo push automatically send upon logging in
        const cancelButton = await duoFrame.$(".btn-cancel");
        if (cancelButton) {
            await cancelButton.click();
        }

        await waitFor(1000);
        // Remember me for 7 days
        await duoFrame.click('#remember_me_label_text');
        await waitFor(1000);
        // Send me a push 
        await duoFrame.click('#auth_methods > fieldset > div.row-label.push-label > button');
    }

    console.info("Logged in successfully.");
    await page.waitForSelector('#startpage-button-go');
    // Get cookies ready to load.
    await page.click('#startpage-button-go');
    const cookies = await page.cookies("https://act.ucsd.edu/webreg2/svc/wradapter/secure/sched-get-schednames?termcode=SP22");
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
    console.log(`Server listening on port ${PORT}`);
});

process.on('SIGTERM', shutDown);
process.on('SIGINT', shutDown);

async function shutDown(): Promise<void> {
    console.log("Shutting down.");
    BROWSER?.close();
    server.close();
}