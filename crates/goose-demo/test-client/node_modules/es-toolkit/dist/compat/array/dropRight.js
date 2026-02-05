'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const dropRight$1 = require('../../array/dropRight.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const toInteger = require('../util/toInteger.js');

function dropRight(collection, itemsCount = 1, guard) {
    if (!isArrayLike.isArrayLike(collection)) {
        return [];
    }
    itemsCount = guard ? 1 : toInteger.toInteger(itemsCount);
    return dropRight$1.dropRight(toArray.toArray(collection), itemsCount);
}

exports.dropRight = dropRight;
