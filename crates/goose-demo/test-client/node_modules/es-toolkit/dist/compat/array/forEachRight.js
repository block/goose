'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const range = require('../../math/range.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function forEachRight(collection, callback = identity.identity) {
    if (!collection) {
        return collection;
    }
    const keys = isArrayLike.isArrayLike(collection) ? range.range(0, collection.length) : Object.keys(collection);
    for (let i = keys.length - 1; i >= 0; i--) {
        const key = keys[i];
        const value = collection[key];
        const result = callback(value, key, collection);
        if (result === false) {
            break;
        }
    }
    return collection;
}

exports.forEachRight = forEachRight;
