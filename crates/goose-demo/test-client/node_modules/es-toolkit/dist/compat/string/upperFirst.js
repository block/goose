'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const upperFirst$1 = require('../../string/upperFirst.js');
const toString = require('../util/toString.js');

function upperFirst(str) {
    return upperFirst$1.upperFirst(toString.toString(str));
}

exports.upperFirst = upperFirst;
