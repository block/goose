'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('../util/toString.js');

function toLower(value) {
    return toString.toString(value).toLowerCase();
}

exports.toLower = toLower;
