'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const take$1 = require('../../array/take.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const toInteger = require('../util/toInteger.js');

function take(arr, count = 1, guard) {
    count = guard ? 1 : toInteger.toInteger(count);
    if (count < 1 || !isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return take$1.take(toArray.toArray(arr), count);
}

exports.take = take;
