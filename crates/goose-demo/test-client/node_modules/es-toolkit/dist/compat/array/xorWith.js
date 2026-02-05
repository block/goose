'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const differenceWith = require('./differenceWith.js');
const intersectionWith = require('./intersectionWith.js');
const last = require('./last.js');
const unionWith = require('./unionWith.js');
const windowed = require('../../array/windowed.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function xorWith(...values) {
    const lastValue = last.last(values);
    let comparator = (a, b) => a === b;
    if (typeof lastValue === 'function') {
        comparator = lastValue;
        values = values.slice(0, -1);
    }
    const arrays = values.filter(isArrayLikeObject.isArrayLikeObject);
    const union = unionWith.unionWith(...arrays, comparator);
    const intersections = windowed.windowed(arrays, 2).map(([arr1, arr2]) => intersectionWith.intersectionWith(arr1, arr2, comparator));
    return differenceWith.differenceWith(union, unionWith.unionWith(...intersections, comparator), comparator);
}

exports.xorWith = xorWith;
