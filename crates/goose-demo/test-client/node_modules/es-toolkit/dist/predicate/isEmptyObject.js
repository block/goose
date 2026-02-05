'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isPlainObject = require('./isPlainObject.js');

function isEmptyObject(value) {
    return isPlainObject.isPlainObject(value) && Object.keys(value).length === 0;
}

exports.isEmptyObject = isEmptyObject;
