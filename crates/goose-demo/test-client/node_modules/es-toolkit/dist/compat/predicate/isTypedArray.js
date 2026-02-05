'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isTypedArray$1 = require('../../predicate/isTypedArray.js');

function isTypedArray(x) {
    return isTypedArray$1.isTypedArray(x);
}

exports.isTypedArray = isTypedArray;
