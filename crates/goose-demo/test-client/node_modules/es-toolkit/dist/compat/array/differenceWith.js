'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const last = require('./last.js');
const difference = require('../../array/difference.js');
const differenceWith$1 = require('../../array/differenceWith.js');
const flattenArrayLike = require('../_internal/flattenArrayLike.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function differenceWith(array, ...values) {
    if (!isArrayLikeObject.isArrayLikeObject(array)) {
        return [];
    }
    const comparator = last.last(values);
    const flattenedValues = flattenArrayLike.flattenArrayLike(values);
    if (typeof comparator === 'function') {
        return differenceWith$1.differenceWith(Array.from(array), flattenedValues, comparator);
    }
    return difference.difference(Array.from(array), flattenedValues);
}

exports.differenceWith = differenceWith;
