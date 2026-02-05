'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isRegExp$1 = require('../../predicate/isRegExp.js');

function isRegExp(value) {
    return isRegExp$1.isRegExp(value);
}

exports.isRegExp = isRegExp;
