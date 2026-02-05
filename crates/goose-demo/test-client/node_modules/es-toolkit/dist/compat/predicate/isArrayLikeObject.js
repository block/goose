'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('./isArrayLike.js');
const isObjectLike = require('./isObjectLike.js');

function isArrayLikeObject(value) {
    return isObjectLike.isObjectLike(value) && isArrayLike.isArrayLike(value);
}

exports.isArrayLikeObject = isArrayLikeObject;
