'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const last = require('./last.js');
const difference = require('../../array/difference.js');
const differenceBy$1 = require('../../array/differenceBy.js');
const flattenArrayLike = require('../_internal/flattenArrayLike.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const iteratee = require('../util/iteratee.js');

function differenceBy(arr, ..._values) {
    if (!isArrayLikeObject.isArrayLikeObject(arr)) {
        return [];
    }
    const iteratee$1 = last.last(_values);
    const values = flattenArrayLike.flattenArrayLike(_values);
    if (isArrayLikeObject.isArrayLikeObject(iteratee$1)) {
        return difference.difference(Array.from(arr), values);
    }
    return differenceBy$1.differenceBy(Array.from(arr), values, iteratee.iteratee(iteratee$1));
}

exports.differenceBy = differenceBy;
