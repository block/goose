'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const uniq$1 = require('../../array/uniq.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function uniq(arr) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return uniq$1.uniq(Array.from(arr));
}

exports.uniq = uniq;
