'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sortedIndexBy = require('./sortedIndexBy.js');

function sortedLastIndexBy(array, value, iteratee) {
    return sortedIndexBy.sortedIndexBy(array, value, iteratee, true);
}

exports.sortedLastIndexBy = sortedLastIndexBy;
