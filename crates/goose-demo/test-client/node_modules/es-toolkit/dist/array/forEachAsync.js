'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const limitAsync = require('./limitAsync.js');

async function forEachAsync(array, callback, options) {
    if (options?.concurrency != null) {
        callback = limitAsync.limitAsync(callback, options.concurrency);
    }
    await Promise.all(array.map(callback));
}

exports.forEachAsync = forEachAsync;
