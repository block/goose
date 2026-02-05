'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sortedLastIndex = require('./sortedLastIndex.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

function sortedLastIndexOf(array, value) {
    if (!array?.length) {
        return -1;
    }
    const index = sortedLastIndex.sortedLastIndex(array, value) - 1;
    if (index >= 0 && isEqualsSameValueZero.isEqualsSameValueZero(array[index], value)) {
        return index;
    }
    return -1;
}

exports.sortedLastIndexOf = sortedLastIndexOf;
