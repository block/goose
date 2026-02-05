'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('./flatten.js');

function flatMap(arr, iteratee, depth = 1) {
    return flatten.flatten(arr.map((item, index) => iteratee(item, index, arr)), depth);
}

exports.flatMap = flatMap;
