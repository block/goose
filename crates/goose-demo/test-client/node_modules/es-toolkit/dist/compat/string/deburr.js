'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const deburr$1 = require('../../string/deburr.js');
const toString = require('../util/toString.js');

function deburr(str) {
    return deburr$1.deburr(toString.toString(str));
}

exports.deburr = deburr;
