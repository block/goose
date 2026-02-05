'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('./flatten.js');
const limitAsync = require('./limitAsync.js');

async function flatMapAsync(array, callback, options) {
    if (options?.concurrency != null) {
        callback = limitAsync.limitAsync(callback, options.concurrency);
    }
    const results = await Promise.all(array.map(callback));
    return flatten.flatten(results);
}

exports.flatMapAsync = flatMapAsync;
