'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isIterateeCall = require('../_internal/isIterateeCall.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const toInteger = require('../util/toInteger.js');

function slice(array, start, end) {
    if (!isArrayLike.isArrayLike(array)) {
        return [];
    }
    const length = array.length;
    if (end === undefined) {
        end = length;
    }
    else if (typeof end !== 'number' && isIterateeCall.isIterateeCall(array, start, end)) {
        start = 0;
        end = length;
    }
    start = toInteger.toInteger(start);
    end = toInteger.toInteger(end);
    if (start < 0) {
        start = Math.max(length + start, 0);
    }
    else {
        start = Math.min(start, length);
    }
    if (end < 0) {
        end = Math.max(length + end, 0);
    }
    else {
        end = Math.min(end, length);
    }
    const resultLength = Math.max(end - start, 0);
    const result = new Array(resultLength);
    for (let i = 0; i < resultLength; ++i) {
        result[i] = array[start + i];
    }
    return result;
}

exports.slice = slice;
