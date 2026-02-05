'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const AbortError = require('../error/AbortError.js');

function delay(ms, { signal } = {}) {
    return new Promise((resolve, reject) => {
        const abortError = () => {
            reject(new AbortError.AbortError());
        };
        const abortHandler = () => {
            clearTimeout(timeoutId);
            abortError();
        };
        if (signal?.aborted) {
            return abortError();
        }
        const timeoutId = setTimeout(() => {
            signal?.removeEventListener('abort', abortHandler);
            resolve();
        }, ms);
        signal?.addEventListener('abort', abortHandler, { once: true });
    });
}

exports.delay = delay;
