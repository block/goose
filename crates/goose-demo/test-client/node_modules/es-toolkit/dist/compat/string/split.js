'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('../util/toString.js');

function split(string, separator, limit) {
    return toString.toString(string).split(separator, limit);
}

exports.split = split;
