'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toNumber = require('../util/toNumber.js');

function delay(func, wait, ...args) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    return setTimeout(func, toNumber.toNumber(wait) || 0, ...args);
}

exports.delay = delay;
