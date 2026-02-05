'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const unescape$1 = require('../../string/unescape.js');
const toString = require('../util/toString.js');

function unescape(str) {
    return unescape$1.unescape(toString.toString(str));
}

exports.unescape = unescape;
