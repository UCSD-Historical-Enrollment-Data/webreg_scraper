export interface IConfiguration {
    terms: string[];
    webhookUrls: WebhookUrl[];
}

export interface WebhookUrl {
    url: string;
    peopleToPing: string[];
}

/**
 * Stops execution of a function for a specified period of time.
 * @param {number} time The time, in milliseconds, to delay execution.
 */
export async function stopFor(time: number): Promise<void> {
    return new Promise(resolve => {
        setTimeout(() => {
            return resolve();
        }, time);
    });
}

/**
 * Gets the current time in a nice string format.
 * @param {Date | number} [date = new Date()] The date to choose, if any.
 * @param {string} [timezone] The timezone, if applicable. Otherwise, GMT is used. See
 * https://en.wikipedia.org/wiki/List_of_tz_database_time_zones for a full list.
 * @returns {string} The current formatter date & time.
 */
export function getDateTime(date: Date | number = new Date(), timezone: string = "America/Los_Angeles"): string {
    if (!isValidTimeZone(timezone)) {
        return new Intl.DateTimeFormat([], {
            year: "numeric",
            month: "numeric",
            day: "numeric",
            hour: "numeric",
            minute: "numeric",
            second: "numeric",
        }).format(date);
    }
    const options: Intl.DateTimeFormatOptions = {
        timeZone: timezone,
        year: "numeric",
        month: "numeric",
        day: "numeric",
        hour: "numeric",
        minute: "numeric",
        second: "numeric",
    };
    return new Intl.DateTimeFormat([], options).format(date);
}

/**
 * Determines whether the given timezone is valid or not.
 * @param {string} tz The timezone to test.
 * @returns {boolean} Whether the timezone is valid.
 * @see https://stackoverflow.com/questions/44115681/javascript-check-if-timezone-name-valid-or-not
 * @see https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
 */
export function isValidTimeZone(tz: string): boolean {
    try {
        Intl.DateTimeFormat(undefined, { timeZone: tz.trim() });
        return true;
    } catch (ex) {
        return false;
    }
}

/**
 * A simple function that attempts to execute a given asynchronous function. This will handle any exceptions that
 * may occur.
 * @param {Function} func The function to run.
 * @return {Promise<T | null>} The result, if any. Null otherwise.
 * @typedef T The function return value.
 */
export async function tryExecuteAsync<T = void>(func: () => Promise<T | null>): Promise<T | null> {
    try {
        return await func();
    } catch (e) {
        return null;
    }
}

/**
 * Gets the current time in a nice string format.
 * @param {Date | number} [date = new Date()] The date to choose, if any.
 * @param {string} [timezone] The timezone, if applicable. Otherwise, GMT is used. See
 * https://en.wikipedia.org/wiki/List_of_tz_database_time_zones for a full list.
 * @returns {string} The current formatter time.
 */
export function getCurrentTime(date: Date | number = new Date(), timezone: string = "America/Los_Angeles"): string {
    if (!isValidTimeZone(timezone)) {
        return new Intl.DateTimeFormat([], {
            hour: "numeric",
            minute: "numeric",
            second: "numeric",
        }).format(date);
    }
    const options: Intl.DateTimeFormatOptions = {
        timeZone: timezone,
        hour: "numeric",
        minute: "numeric",
        second: "numeric",
    };
    return new Intl.DateTimeFormat([], options).format(date);
}