import axios, { AxiosInstance } from "axios";
import * as fs from "fs";
import * as path from "path";
import { getCurrentTime, getDateTime, IConfiguration, stopFor, tryExecuteAsync } from "./util";

const AXIOS: AxiosInstance = axios.create();
const COOLDOWN: number = 90 * 1000;
const DIVIDER: string = "========================================================";

async function main() {
    // Validate that everything is correct.
    const content = fs.readFileSync(path.join(__dirname, "..", "config.json"));
    const config: IConfiguration = JSON.parse(content.toString());

    if ((config.terms?.length ?? 0) === 0) {
        console.error("'terms' is required but not defined in configuration file.");
        return;
    }

    if ((config.webhookUrls?.length ?? 0) === 0) {
        console.error("'webhookUrls' is required but not defined in configuration file.");
        return;
    }

    for (const term of config.terms) {
        check(term, config).then();
    }
}

async function check(term: string, config: IConfiguration): Promise<void> {
    const currDate = new Date();
    const dateStr = getDateTime(currDate);
    if (currDate.getHours() === 4) {
        console.info(`[${dateStr}] [${term}] Skipping check due to time being during WebReg restart period.`);
        await stopFor(COOLDOWN);
        check(term, config).then();
        return;
    }

    console.info(`[${dateStr}] [${term}] Checking scraper status (hour: ${currDate.getHours()}).`);

    // Check the status of each endpoint
    const lookupRes: unknown[] | { "error": string } | null = await tryExecuteAsync(async () => {
        const data = await AXIOS.get(`http://127.0.0.1:8000/course/${term}/CSE/8A`);
        return data.data;
    });

    const statusRes: { "status": boolean } | { "error": string } | null = await tryExecuteAsync(async () => {
        const data = await AXIOS.get(`http://127.0.0.1:8000/status/${term}`);
        return data.data;
    });

    // Look up result should be straightforward
    let strToSend: string[] = [];
    if (!Array.isArray(lookupRes)) {
        if (lookupRes) {
            strToSend.push(
                "⚠️ `[Lookup]` The scraper returned an error message: ```",
                lookupRes.error,
                "```"
            );
        }
        else {
            strToSend.push(
                "❌ `[Lookup]` The scraper did not respond to the lookup request and might be down.",
            );
        }
    }

    // Status might not be.
    if (statusRes) {
        if ("status" in statusRes && !statusRes.status) {
            strToSend.push(
                "⚠️ `[Status]` The scraper is currently not running.");
        }
        else if ("error" in statusRes) {
            strToSend.push(
                "⚠️ `[Status]` The scraper returned an error message: ```",
                statusRes.error,
                "```"
            );
        }
    }
    else {
        strToSend.push(
            "❌ `[Status]` The scraper did not respond to the status request and might be down.",
        );
    }

    if (strToSend.length > 0) {
        const baseMsg = `**\`[${term} • ${getCurrentTime()}]\`** __**Scraper Warning**__\n${strToSend.join("\n")}`;
        for (const {url, peopleToPing} of config.webhookUrls) {
            let actualMsg = baseMsg;
            if (peopleToPing.length > 0) {
                actualMsg += "\n" + (peopleToPing.map(x => `<@${x}>`).join(", "));
            }
            
            actualMsg += `\n${DIVIDER}`;

            try {
                await AXIOS.post(url, {
                    "content": actualMsg,
                });
            }
            catch (e) {
                console.error(`[${dateStr}] [${term}] Unable to send error information to: ${url}\n${e}\n${DIVIDER}`);
            }
        }
    }

    await stopFor(COOLDOWN);
    check(term, config).then();
}



main().then();