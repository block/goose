'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const unzip$1 = require('../../array/unzip.js');
const isArray = require('../predicate/isArray.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function unzip(array) {
    if (!isArrayLikeObject.isArrayLikeObject(array) || !array.length) {
        return [];
    }
    array = isArray.isArray(array) ? array : Array.from(array);
    array = array.filter(item => isArrayLikeObject.isArrayLikeObject(item));
    return unzip$1.unzip(array);
}

exports.unzip = unzip;
