'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function filter(set, callback) {
    const result = new Set();
    for (const value of set) {
        if (callback(value, value, set)) {
            result.add(value);
        }
    }
    return result;
}

exports.filter = filter;
