'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('./flatten.js');

function flattenDeep(arr) {
    return flatten.flatten(arr, Infinity);
}

exports.flattenDeep = flattenDeep;
