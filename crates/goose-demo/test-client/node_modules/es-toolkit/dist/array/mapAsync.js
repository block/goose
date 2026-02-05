'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const limitAsync = require('./limitAsync.js');

function mapAsync(array, callback, options) {
    if (options?.concurrency != null) {
        callback = limitAsync.limitAsync(callback, options.concurrency);
    }
    return Promise.all(array.map(callback));
}

exports.mapAsync = mapAsync;
