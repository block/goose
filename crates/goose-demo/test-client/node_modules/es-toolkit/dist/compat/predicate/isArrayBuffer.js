'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayBuffer$1 = require('../../predicate/isArrayBuffer.js');

function isArrayBuffer(value) {
    return isArrayBuffer$1.isArrayBuffer(value);
}

exports.isArrayBuffer = isArrayBuffer;
