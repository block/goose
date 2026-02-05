'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('../predicate/isArrayLike.js');
const isMap = require('../predicate/isMap.js');

function toArray(value) {
    if (value == null) {
        return [];
    }
    if (isArrayLike.isArrayLike(value) || isMap.isMap(value)) {
        return Array.from(value);
    }
    if (typeof value === 'object') {
        return Object.values(value);
    }
    return [];
}

exports.toArray = toArray;
