'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('../predicate/isArrayLike.js');

function indexOf(array, searchElement, fromIndex) {
    if (!isArrayLike.isArrayLike(array)) {
        return -1;
    }
    if (Number.isNaN(searchElement)) {
        fromIndex = fromIndex ?? 0;
        if (fromIndex < 0) {
            fromIndex = Math.max(0, array.length + fromIndex);
        }
        for (let i = fromIndex; i < array.length; i++) {
            if (Number.isNaN(array[i])) {
                return i;
            }
        }
        return -1;
    }
    return Array.from(array).indexOf(searchElement, fromIndex);
}

exports.indexOf = indexOf;
