'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const range = require('../../math/range.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function forEach(collection, callback = identity.identity) {
    if (!collection) {
        return collection;
    }
    const keys = isArrayLike.isArrayLike(collection) || Array.isArray(collection) ? range.range(0, collection.length) : Object.keys(collection);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = collection[key];
        const result = callback(value, key, collection);
        if (result === false) {
            break;
        }
    }
    return collection;
}

exports.forEach = forEach;
