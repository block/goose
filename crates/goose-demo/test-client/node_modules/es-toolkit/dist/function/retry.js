'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const delay = require('../promise/delay.js');

const DEFAULT_DELAY = 0;
const DEFAULT_RETRIES = Number.POSITIVE_INFINITY;
const DEFAULT_SHOULD_RETRY = () => true;
async function retry(func, _options) {
    let delay$1;
    let retries;
    let signal;
    let shouldRetry;
    if (typeof _options === 'number') {
        delay$1 = DEFAULT_DELAY;
        retries = _options;
        signal = undefined;
        shouldRetry = DEFAULT_SHOULD_RETRY;
    }
    else {
        delay$1 = _options?.delay ?? DEFAULT_DELAY;
        retries = _options?.retries ?? DEFAULT_RETRIES;
        signal = _options?.signal;
        shouldRetry = _options?.shouldRetry ?? DEFAULT_SHOULD_RETRY;
    }
    let error;
    for (let attempts = 0; attempts < retries; attempts++) {
        if (signal?.aborted) {
            throw error ?? new Error(`The retry operation was aborted due to an abort signal.`);
        }
        try {
            return await func();
        }
        catch (err) {
            error = err;
            if (!shouldRetry(err, attempts)) {
                throw err;
            }
            const currentDelay = typeof delay$1 === 'function' ? delay$1(attempts) : delay$1;
            await delay.delay(currentDelay);
        }
    }
    throw error;
}

exports.retry = retry;
