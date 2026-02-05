'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isWeakMap$1 = require('../../predicate/isWeakMap.js');

function isWeakMap(value) {
    return isWeakMap$1.isWeakMap(value);
}

exports.isWeakMap = isWeakMap;
