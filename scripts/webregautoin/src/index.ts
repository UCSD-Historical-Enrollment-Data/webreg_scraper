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
import { parseArgs } from 'node:util';
import { PUSH, fetchCookies, getTermSeqId, logNice, printHelpMessage } from "./fns";
import { IConfig, Context, ITermInfo } from "./types";
import { createInterface } from "readline";
import { Writable } from "stream";
import prompts = require("prompts");

async function credentialPrompt() {
    console.log("\n============================================================");
    console.log(" Waiting for credentials.");
    console.log(" If running detached, attach to this container to continue.");
    console.log("============================================================\n");

    const response = await prompts([
        {
            type: 'text',
            name: 'username',
            message: 'Enter your TritonLink Username:'
        },
        {
            type: 'password',
            name: 'password',
            message: 'Enter your TritonLink Password:',
            mask: '*' 
        }
    ]);

    console.log("\nCredentials saved to RAM. Starting server...");
    
    return { 
        username: response.username, 
        password: response.password 
    };
}


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
        // If debug mode is true, turn OFF headless mode
        headless: !debug
    });

    // moved to env vars for "better" security (not really, but it's something)
    let username = process.env.WEBREG_USERNAME || "";
    let password = process.env.WEBREG_PASSWORD || "";

    if (!username || !password) {
        const credentials = await credentialPrompt();
        username = credentials.username;
        password = credentials.password;
    }

    const config: IConfig = {
        webreg: {
            username: username,
            password: password
        },
        settings: {
            loginType: process.env.LOGIN_TYPE || "push",
            automaticPushEnabled: process.env.AUTOMATIC_PUSH_ENABLED === "true"
        }
    };

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

    let context: Context;
    if (config.settings.loginType === PUSH) {
        context = {
            webreg: config.webreg,
            session: {
                start: 0,
                callHistory: []
            },
            termInfo,
            automaticPushEnabled: config.settings.automaticPushEnabled,
            loginType: PUSH
        };
    }
    else {
        console.error("Your login type must either be 'sms' or 'push'");
        process.exit(1);
    }

    let isReady = false;

    // Very basic server.
    const server = http.createServer(async (req, res) => {
        if (req.url === "/healthy") {
            if (isReady) {
                res.writeHead(200, { "Content-Type": "application/json" });
                res.end(
                    JSON.stringify({
                        status: "ok"
                    })
                );
            } else {
                res.writeHead(503, { "Content-Type": "application/json" });
                res.end(
                    JSON.stringify({
                        status: "waiting for login"
                    })
                );
            }
            return;
        }

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
                    cookie: await fetchCookies(context, browser, false)
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
    try {
        logNice("Init", "Performing initial health check...");
        await fetchCookies(context, browser, true);
        isReady = true;
    } catch (e) {
        logNice("Init", "Health check failed. Restart container and try again.");
        logNice("Init", "Error details:");
        console.error(e);
    }
}

main().then();