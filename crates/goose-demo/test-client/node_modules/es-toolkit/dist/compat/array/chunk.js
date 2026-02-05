'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const chunk$1 = require('../../array/chunk.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function chunk(arr, size = 1) {
    size = Math.max(Math.floor(size), 0);
    if (size === 0 || !isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return chunk$1.chunk(toArray.toArray(arr), size);
}

exports.chunk = chunk;
