'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isBuffer$1 = require('../../predicate/isBuffer.js');

function isBuffer(x) {
    return isBuffer$1.isBuffer(x);
}

exports.isBuffer = isBuffer;
