'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const difference = require('./difference.js');
const intersection = require('./intersection.js');
const union = require('./union.js');

function xor(arr1, arr2) {
    return difference.difference(union.union(arr1, arr2), intersection.intersection(arr1, arr2));
}

exports.xor = xor;
