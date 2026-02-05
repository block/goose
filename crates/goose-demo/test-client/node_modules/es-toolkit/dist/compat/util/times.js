'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toInteger = require('./toInteger.js');

function times(n, getValue) {
    n = toInteger.toInteger(n);
    if (n < 1 || !Number.isSafeInteger(n)) {
        return [];
    }
    const result = new Array(n);
    for (let i = 0; i < n; i++) {
        result[i] = typeof getValue === 'function' ? getValue(i) : i;
    }
    return result;
}

exports.times = times;
