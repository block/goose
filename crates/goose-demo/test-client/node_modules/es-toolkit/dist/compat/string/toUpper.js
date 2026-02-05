'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('../util/toString.js');

function toUpper(value) {
    return toString.toString(value).toUpperCase();
}

exports.toUpper = toUpper;
