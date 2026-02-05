'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const differenceBy = require('./differenceBy.js');
const intersectionBy = require('./intersectionBy.js');
const unionBy = require('./unionBy.js');

function xorBy(arr1, arr2, mapper) {
    const union = unionBy.unionBy(arr1, arr2, mapper);
    const intersection = intersectionBy.intersectionBy(arr1, arr2, mapper);
    return differenceBy.differenceBy(union, intersection, mapper);
}

exports.xorBy = xorBy;
