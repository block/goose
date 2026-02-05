'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function before(n, func) {
    if (!Number.isInteger(n) || n < 0) {
        throw new Error('n must be a non-negative integer.');
    }
    let counter = 0;
    return (...args) => {
        if (++counter < n) {
            return func(...args);
        }
        return undefined;
    };
}

exports.before = before;
