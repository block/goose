'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const range = require('../../math/range.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function reduce(collection, iteratee = identity.identity, accumulator) {
    if (!collection) {
        return accumulator;
    }
    let keys;
    let startIndex = 0;
    if (isArrayLike.isArrayLike(collection)) {
        keys = range.range(0, collection.length);
        if (accumulator == null && collection.length > 0) {
            accumulator = collection[0];
            startIndex += 1;
        }
    }
    else {
        keys = Object.keys(collection);
        if (accumulator == null) {
            accumulator = collection[keys[0]];
            startIndex += 1;
        }
    }
    for (let i = startIndex; i < keys.length; i++) {
        const key = keys[i];
        const value = collection[key];
        accumulator = iteratee(accumulator, value, key, collection);
    }
    return accumulator;
}

exports.reduce = reduce;
