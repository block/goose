'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const compact$1 = require('../../array/compact.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function compact(arr) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return compact$1.compact(Array.from(arr));
}

exports.compact = compact;
