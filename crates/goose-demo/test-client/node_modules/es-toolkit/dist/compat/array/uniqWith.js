'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const uniqWith$1 = require('../../array/uniqWith.js');
const uniq = require('./uniq.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function uniqWith(arr, comparator) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return typeof comparator === 'function' ? uniqWith$1.uniqWith(Array.from(arr), comparator) : uniq.uniq(Array.from(arr));
}

exports.uniqWith = uniqWith;
