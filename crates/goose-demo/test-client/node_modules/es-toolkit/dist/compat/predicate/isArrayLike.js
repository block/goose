'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isLength = require('../../predicate/isLength.js');

function isArrayLike(value) {
    return value != null && typeof value !== 'function' && isLength.isLength(value.length);
}

exports.isArrayLike = isArrayLike;
