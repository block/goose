'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const initial$1 = require('../../array/initial.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function initial(arr) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return initial$1.initial(Array.from(arr));
}

exports.initial = initial;
