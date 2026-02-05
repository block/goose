'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const last = require('../../array/last.js');
const uniq = require('../../array/uniq.js');
const uniqBy = require('../../array/uniqBy.js');
const ary = require('../../function/ary.js');
const flattenArrayLike = require('../_internal/flattenArrayLike.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const iteratee = require('../util/iteratee.js');

function unionBy(...values) {
    const lastValue = last.last(values);
    const flattened = flattenArrayLike.flattenArrayLike(values);
    if (isArrayLikeObject.isArrayLikeObject(lastValue) || lastValue == null) {
        return uniq.uniq(flattened);
    }
    return uniqBy.uniqBy(flattened, ary.ary(iteratee.iteratee(lastValue), 1));
}

exports.unionBy = unionBy;
