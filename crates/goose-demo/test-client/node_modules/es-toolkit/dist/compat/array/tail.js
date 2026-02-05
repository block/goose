'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const tail$1 = require('../../array/tail.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function tail(arr) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return tail$1.tail(toArray.toArray(arr));
}

exports.tail = tail;
