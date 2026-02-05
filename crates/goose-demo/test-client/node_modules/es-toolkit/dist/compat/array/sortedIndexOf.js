'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sortedIndex = require('./sortedIndex.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

function sortedIndexOf(array, value) {
    if (!array?.length) {
        return -1;
    }
    const index = sortedIndex.sortedIndex(array, value);
    if (index < array.length && isEqualsSameValueZero.isEqualsSameValueZero(array[index], value)) {
        return index;
    }
    return -1;
}

exports.sortedIndexOf = sortedIndexOf;
