'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const last = require('../../array/last.js');
const uniq = require('../../array/uniq.js');
const uniqWith = require('../../array/uniqWith.js');
const flattenArrayLike = require('../_internal/flattenArrayLike.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function unionWith(...values) {
    const lastValue = last.last(values);
    const flattened = flattenArrayLike.flattenArrayLike(values);
    if (isArrayLikeObject.isArrayLikeObject(lastValue) || lastValue == null) {
        return uniq.uniq(flattened);
    }
    return uniqWith.uniqWith(flattened, lastValue);
}

exports.unionWith = unionWith;
