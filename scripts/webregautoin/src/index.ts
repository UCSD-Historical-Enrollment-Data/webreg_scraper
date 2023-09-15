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
import {parseArgs} from 'node:util';
import {fetchCookies, getTermSeqId, logNice, printHelpMessage} from "./fns";
import {IContext, ICredentials, ITermInfo} from "./types";

async function main(): Promise<void> {
    const args = parseArgs({
        options: {
            port: {
                type: "string",
                short: "p"
            },
            term: {
                type: "string",
                short: "t"
            },
            debug: {
                type: "boolean",
                short: "d"
            }
        }
    });

    const port = Number.parseInt(args.values.port ?? "0", 10);
    if (port === 0) {
        printHelpMessage();
        process.exit(1);
    }

    const debug = args.values.debug ?? false;
    let browser: puppeteer.Browser = await puppeteer.launch({
        args: ['--no-sandbox', '--disable-setuid-sandbox'],
        // If debug mode is on, turn OFF headless mode
        headless: !debug
    });

    const credentials: ICredentials = JSON.parse(
        fs.readFileSync(path.join(__dirname, "..", "credentials.json")).toString());

    const term = args.values.term?.toUpperCase();
    let termInfo: ITermInfo | null = null;
    if (term) {
        const seqId = getTermSeqId(term);
        if (seqId !== 0) {
            termInfo = {
                termName: term,
                seqId
            };
        }
    }

    const context: IContext = {
        credentials,
        session: {
            start: 0,
            callHistory: []
        },
        termInfo
    };

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
                    cookie: await fetchCookies(context, browser)
                })
            );
        } else if (req.url === "/history") {
            res.end(
                JSON.stringify(context.session.callHistory)
            );
        } else if (req.url === "/start") {
            res.end(
                JSON.stringify(context.session.start)
            );
        } else {
            res.end(
                JSON.stringify({
                    error: http.STATUS_CODES[404]
                })
            );
        }
    });

    server.listen(port, () => {
        logNice("Init", `Server listening on port ${port}`);
    });

    process.on('SIGTERM', shutDown);
    process.on('SIGINT', shutDown);

    async function shutDown(): Promise<void> {
        logNice("ShutDown", "Shutting down server & closing browser.");
        browser?.close();
        server.close();
    }

    // Initial warmup call.
    await fetchCookies(context, browser);
}

main().then();