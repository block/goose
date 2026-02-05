'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const last$1 = require('../../array/last.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function last(array) {
    if (!isArrayLike.isArrayLike(array)) {
        return undefined;
    }
    return last$1.last(toArray.toArray(array));
}

exports.last = last;
