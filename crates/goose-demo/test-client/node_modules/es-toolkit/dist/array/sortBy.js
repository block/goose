'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const orderBy = require('./orderBy.js');

function sortBy(arr, criteria) {
    return orderBy.orderBy(arr, criteria, ['asc']);
}

exports.sortBy = sortBy;
