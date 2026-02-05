'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flattenDepth = require('./flattenDepth.js');

function flattenDeep(value) {
    return flattenDepth.flattenDepth(value, Infinity);
}

exports.flattenDeep = flattenDeep;
