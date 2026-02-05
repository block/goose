'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const drop$1 = require('../../array/drop.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const toInteger = require('../util/toInteger.js');

function drop(collection, itemsCount = 1, guard) {
    if (!isArrayLike.isArrayLike(collection)) {
        return [];
    }
    itemsCount = guard ? 1 : toInteger.toInteger(itemsCount);
    return drop$1.drop(toArray.toArray(collection), itemsCount);
}

exports.drop = drop;
