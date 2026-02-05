'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isMap$1 = require('../../predicate/isMap.js');

function isMap(value) {
    return isMap$1.isMap(value);
}

exports.isMap = isMap;
