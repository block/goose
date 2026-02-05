'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('../util/toString.js');

function padStart(str, length = 0, chars = ' ') {
    return toString.toString(str).padStart(length, chars);
}

exports.padStart = padStart;
