'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const without$1 = require('../../array/without.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function without(array, ...values) {
    if (!isArrayLikeObject.isArrayLikeObject(array)) {
        return [];
    }
    return without$1.without(Array.from(array), ...values);
}

exports.without = without;
