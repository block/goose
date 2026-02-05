'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('../predicate/isArrayLike.js');

function lastIndexOf(array, searchElement, fromIndex) {
    if (!isArrayLike.isArrayLike(array) || array.length === 0) {
        return -1;
    }
    const length = array.length;
    let index = fromIndex ?? length - 1;
    if (fromIndex != null) {
        index = index < 0 ? Math.max(length + index, 0) : Math.min(index, length - 1);
    }
    if (Number.isNaN(searchElement)) {
        for (let i = index; i >= 0; i--) {
            if (Number.isNaN(array[i])) {
                return i;
            }
        }
    }
    return Array.from(array).lastIndexOf(searchElement, index);
}

exports.lastIndexOf = lastIndexOf;
