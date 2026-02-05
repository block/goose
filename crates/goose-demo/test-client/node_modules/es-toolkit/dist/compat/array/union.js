'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatMapDepth = require('./flatMapDepth.js');
const uniq = require('../../array/uniq.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function union(...arrays) {
    const validArrays = arrays.filter(isArrayLikeObject.isArrayLikeObject);
    const flattened = flatMapDepth.flatMapDepth(validArrays, v => Array.from(v), 1);
    return uniq.uniq(flattened);
}

exports.union = union;
