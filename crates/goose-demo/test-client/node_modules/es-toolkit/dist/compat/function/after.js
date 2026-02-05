'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toInteger = require('../util/toInteger.js');

function after(n, func) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    n = toInteger.toInteger(n);
    return function (...args) {
        if (--n < 1) {
            return func.apply(this, args);
        }
    };
}

exports.after = after;
