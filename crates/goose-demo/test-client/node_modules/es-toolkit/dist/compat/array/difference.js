'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const difference$1 = require('../../array/difference.js');
const toArray = require('../_internal/toArray.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function difference(arr, ...values) {
    if (!isArrayLikeObject.isArrayLikeObject(arr)) {
        return [];
    }
    const arr1 = toArray.toArray(arr);
    const arr2 = [];
    for (let i = 0; i < values.length; i++) {
        const value = values[i];
        if (isArrayLikeObject.isArrayLikeObject(value)) {
            arr2.push(...Array.from(value));
        }
    }
    return difference$1.difference(arr1, arr2);
}

exports.difference = difference;
