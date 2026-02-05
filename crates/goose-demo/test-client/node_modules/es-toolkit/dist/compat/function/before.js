'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toInteger = require('../util/toInteger.js');

function before(n, func) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    let result;
    n = toInteger.toInteger(n);
    return function (...args) {
        if (--n > 0) {
            result = func.apply(this, args);
        }
        if (n <= 1 && func) {
            func = undefined;
        }
        return result;
    };
}

exports.before = before;
