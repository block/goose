'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const range = require('../../math/range.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const iteratee = require('../util/iteratee.js');

function map(collection, _iteratee) {
    if (!collection) {
        return [];
    }
    const keys = isArrayLike.isArrayLike(collection) || Array.isArray(collection) ? range.range(0, collection.length) : Object.keys(collection);
    const iteratee$1 = iteratee.iteratee(_iteratee ?? identity.identity);
    const result = new Array(keys.length);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = collection[key];
        result[i] = iteratee$1(value, key, collection);
    }
    return result;
}

exports.map = map;
