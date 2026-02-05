'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const semaphore = require('../promise/semaphore.js');

function limitAsync(callback, concurrency) {
    const semaphore$1 = new semaphore.Semaphore(concurrency);
    return async function (...args) {
        try {
            await semaphore$1.acquire();
            return await callback.apply(this, args);
        }
        finally {
            semaphore$1.release();
        }
    };
}

exports.limitAsync = limitAsync;
