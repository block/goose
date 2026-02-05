'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const takeRight$1 = require('../../array/takeRight.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const toInteger = require('../util/toInteger.js');

function takeRight(arr, count = 1, guard) {
    count = guard ? 1 : toInteger.toInteger(count);
    if (count <= 0 || !isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return takeRight$1.takeRight(toArray.toArray(arr), count);
}

exports.takeRight = takeRight;
