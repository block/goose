'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const uniqBy = require('./uniqBy.js');

function unionBy(arr1, arr2, mapper) {
    return uniqBy.uniqBy(arr1.concat(arr2), mapper);
}

exports.unionBy = unionBy;
