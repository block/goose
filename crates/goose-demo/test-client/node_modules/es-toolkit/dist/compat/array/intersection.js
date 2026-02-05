'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const intersection$1 = require('../../array/intersection.js');
const uniq = require('../../array/uniq.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function intersection(...arrays) {
    if (arrays.length === 0) {
        return [];
    }
    if (!isArrayLikeObject.isArrayLikeObject(arrays[0])) {
        return [];
    }
    let result = uniq.uniq(Array.from(arrays[0]));
    for (let i = 1; i < arrays.length; i++) {
        const array = arrays[i];
        if (!isArrayLikeObject.isArrayLikeObject(array)) {
            return [];
        }
        result = intersection$1.intersection(result, Array.from(array));
    }
    return result;
}

exports.intersection = intersection;
