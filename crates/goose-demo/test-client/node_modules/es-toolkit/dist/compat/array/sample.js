'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sample$1 = require('../../array/sample.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function sample(collection) {
    if (collection == null) {
        return undefined;
    }
    if (isArrayLike.isArrayLike(collection)) {
        return sample$1.sample(toArray.toArray(collection));
    }
    return sample$1.sample(Object.values(collection));
}

exports.sample = sample;
