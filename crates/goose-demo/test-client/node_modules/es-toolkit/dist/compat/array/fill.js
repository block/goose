'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const fill$1 = require('../../array/fill.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isString = require('../predicate/isString.js');

function fill(array, value, start = 0, end = array ? array.length : 0) {
    if (!isArrayLike.isArrayLike(array)) {
        return [];
    }
    if (isString.isString(array)) {
        return array;
    }
    start = Math.floor(start);
    end = Math.floor(end);
    if (!start) {
        start = 0;
    }
    if (!end) {
        end = 0;
    }
    return fill$1.fill(array, value, start, end);
}

exports.fill = fill;
