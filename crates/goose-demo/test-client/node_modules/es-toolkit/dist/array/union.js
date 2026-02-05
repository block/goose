'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const uniq = require('./uniq.js');

function union(arr1, arr2) {
    return uniq.uniq(arr1.concat(arr2));
}

exports.union = union;
