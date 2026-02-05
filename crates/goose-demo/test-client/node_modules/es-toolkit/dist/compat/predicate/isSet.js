'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isSet$1 = require('../../predicate/isSet.js');

function isSet(value) {
    return isSet$1.isSet(value);
}

exports.isSet = isSet;
