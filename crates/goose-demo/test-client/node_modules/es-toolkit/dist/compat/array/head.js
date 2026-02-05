'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const head$1 = require('../../array/head.js');
const toArray = require('../_internal/toArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function head(arr) {
    if (!isArrayLike.isArrayLike(arr)) {
        return undefined;
    }
    return head$1.head(toArray.toArray(arr));
}

exports.head = head;
