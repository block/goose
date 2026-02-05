'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function flattenArrayLike(values) {
    const result = [];
    for (let i = 0; i < values.length; i++) {
        const arrayLike = values[i];
        if (!isArrayLikeObject.isArrayLikeObject(arrayLike)) {
            continue;
        }
        for (let j = 0; j < arrayLike.length; j++) {
            result.push(arrayLike[j]);
        }
    }
    return result;
}

exports.flattenArrayLike = flattenArrayLike;
