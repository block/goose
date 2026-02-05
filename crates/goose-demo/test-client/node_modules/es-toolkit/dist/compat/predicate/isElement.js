'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isObjectLike = require('./isObjectLike.js');
const isPlainObject = require('./isPlainObject.js');

function isElement(value) {
    return isObjectLike.isObjectLike(value) && value.nodeType === 1 && !isPlainObject.isPlainObject(value);
}

exports.isElement = isElement;
