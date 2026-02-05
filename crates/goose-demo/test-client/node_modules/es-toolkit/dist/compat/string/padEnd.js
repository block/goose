'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('../util/toString.js');

function padEnd(str, length = 0, chars = ' ') {
    return toString.toString(str).padEnd(length, chars);
}

exports.padEnd = padEnd;
