'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const escape$1 = require('../../string/escape.js');
const toString = require('../util/toString.js');

function escape(string) {
    return escape$1.escape(toString.toString(string));
}

exports.escape = escape;
