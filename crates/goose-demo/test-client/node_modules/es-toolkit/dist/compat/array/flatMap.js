'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flattenDepth = require('./flattenDepth.js');
const map = require('./map.js');
const isNil = require('../../predicate/isNil.js');

function flatMap(collection, iteratee) {
    if (isNil.isNil(collection)) {
        return [];
    }
    const mapped = isNil.isNil(iteratee) ? map.map(collection) : map.map(collection, iteratee);
    return flattenDepth.flattenDepth(mapped, 1);
}

exports.flatMap = flatMap;
