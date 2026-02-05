'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const differenceBy = require('./differenceBy.js');
const intersectionBy = require('./intersectionBy.js');
const last = require('./last.js');
const unionBy = require('./unionBy.js');
const windowed = require('../../array/windowed.js');
const identity = require('../../function/identity.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const iteratee = require('../util/iteratee.js');

function xorBy(...values) {
    const lastValue = last.last(values);
    let mapper = identity.identity;
    if (!isArrayLikeObject.isArrayLikeObject(lastValue) && lastValue != null) {
        mapper = iteratee.iteratee(lastValue);
        values = values.slice(0, -1);
    }
    const arrays = values.filter(isArrayLikeObject.isArrayLikeObject);
    const union = unionBy.unionBy(...arrays, mapper);
    const intersections = windowed.windowed(arrays, 2).map(([arr1, arr2]) => intersectionBy.intersectionBy(arr1, arr2, mapper));
    return differenceBy.differenceBy(union, unionBy.unionBy(...intersections, mapper), mapper);
}

exports.xorBy = xorBy;
