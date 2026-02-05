'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isDate$1 = require('../../predicate/isDate.js');

function isDate(value) {
    return isDate$1.isDate(value);
}

exports.isDate = isDate;
