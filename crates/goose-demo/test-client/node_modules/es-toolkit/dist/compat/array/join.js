'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('../predicate/isArrayLike.js');

function join(array, separator) {
    if (!isArrayLike.isArrayLike(array)) {
        return '';
    }
    return Array.from(array).join(separator);
}

exports.join = join;
