'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const unzip = require('../../array/unzip.js');
const isArray = require('../predicate/isArray.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function unzipWith(array, iteratee) {
    if (!isArrayLikeObject.isArrayLikeObject(array) || !array.length) {
        return [];
    }
    const unzipped = isArray.isArray(array) ? unzip.unzip(array) : unzip.unzip(Array.from(array, value => Array.from(value)));
    if (!iteratee) {
        return unzipped;
    }
    const result = new Array(unzipped.length);
    for (let i = 0; i < unzipped.length; i++) {
        const value = unzipped[i];
        result[i] = iteratee(...value);
    }
    return result;
}

exports.unzipWith = unzipWith;
