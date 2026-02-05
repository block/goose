'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const toInteger = require('../util/toInteger.js');

function nth(array, n = 0) {
    if (!isArrayLikeObject.isArrayLikeObject(array) || array.length === 0) {
        return undefined;
    }
    n = toInteger.toInteger(n);
    if (n < 0) {
        n += array.length;
    }
    return array[n];
}

exports.nth = nth;
