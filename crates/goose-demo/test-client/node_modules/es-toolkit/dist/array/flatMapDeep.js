'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flattenDeep = require('./flattenDeep.js');

function flatMapDeep(arr, iteratee) {
    return flattenDeep.flattenDeep(arr.map((item, index) => iteratee(item, index, arr)));
}

exports.flatMapDeep = flatMapDeep;
