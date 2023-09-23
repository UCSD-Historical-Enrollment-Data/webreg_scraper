import axios, { AxiosInstance } from "axios";
import * as fs from "fs";
import * as path from "path";
import { getCurrentTime, getDateTime, IConfiguration, stopFor, tryExecuteAsync } from "./util";

const AXIOS: AxiosInstance = axios.create();
const COOLDOWN: number = 5 * 60 * 1000;
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
        let data = await AXIOS.get(`http://127.0.0.1:3000/live/${term}/course_info?subject=CSE&number=8A`, {
            headers: {
                "Authorization": `Bearer ${config.apiKey}`
            }
        });

        return data.data;
    });

    const statusRes: { "api": boolean } | { "error": string } | null = await tryExecuteAsync(async () => {
        const data = await AXIOS.get(`http://127.0.0.1:3000/health`, {
            headers: {
                "Authorization": `Bearer ${config.apiKey}`
            }
        });

        return data.data;
    });

    // Look up result should be straightforward
    let strToSend: string[] = [];
    if (!Array.isArray(lookupRes)) {
        if (lookupRes) {
            strToSend.push(
                "⚠️ `[Lookup/8A]` The scraper returned an error message: ```",
                lookupRes.error,
                "```"
            );
        }
        else {
            strToSend.push(
                "❌ `[Lookup/8A]` The scraper did not respond to the lookup request and might be down.",
            );
        }
    }

    // Status might not be.
    if (statusRes) {
        if ("api" in statusRes && !statusRes.api) {
            strToSend.push(
                "⚠️ `[Status/Health]` The scraper is currently not running.");
        }
        else if ("error" in statusRes) {
            strToSend.push(
                "⚠️ `[Status/Health]` The scraper returned an error message: ```",
                statusRes.error,
                "```"
            );
        }
    }
    else {
        strToSend.push(
            "❌ `[Status/Health]` The scraper did not respond to the status request and might be down.",
        );
    }

    if (strToSend.length > 0) {
        const baseMsg = `# **\`[${term} • ${getCurrentTime()}]\`** __**"something brokeeeeeeeee" - ruby**__\n${strToSend.join("\n")}`;
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