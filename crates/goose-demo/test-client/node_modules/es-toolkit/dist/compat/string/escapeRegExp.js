'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const escapeRegExp$1 = require('../../string/escapeRegExp.js');
const toString = require('../util/toString.js');

function escapeRegExp(str) {
    return escapeRegExp$1.escapeRegExp(toString.toString(str));
}

exports.escapeRegExp = escapeRegExp;
