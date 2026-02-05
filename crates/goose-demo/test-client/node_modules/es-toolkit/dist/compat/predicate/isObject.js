'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isObject(value) {
    return value !== null && (typeof value === 'object' || typeof value === 'function');
}

exports.isObject = isObject;
