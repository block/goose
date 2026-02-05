'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const limitAsync = require('./limitAsync.js');

async function filterAsync(array, predicate, options) {
    if (options?.concurrency != null) {
        predicate = limitAsync.limitAsync(predicate, options.concurrency);
    }
    const results = await Promise.all(array.map(predicate));
    return array.filter((_, index) => results[index]);
}

exports.filterAsync = filterAsync;
