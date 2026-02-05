'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const lowerFirst$1 = require('../../string/lowerFirst.js');
const toString = require('../util/toString.js');

function lowerFirst(str) {
    return lowerFirst$1.lowerFirst(toString.toString(str));
}

exports.lowerFirst = lowerFirst;
