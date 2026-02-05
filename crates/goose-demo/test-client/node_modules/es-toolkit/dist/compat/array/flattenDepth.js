'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('./flatten.js');

function flattenDepth(array, depth = 1) {
    return flatten.flatten(array, depth);
}

exports.flattenDepth = flattenDepth;
