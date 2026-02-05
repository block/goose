'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isWeakSet$1 = require('../../predicate/isWeakSet.js');

function isWeakSet(value) {
    return isWeakSet$1.isWeakSet(value);
}

exports.isWeakSet = isWeakSet;
