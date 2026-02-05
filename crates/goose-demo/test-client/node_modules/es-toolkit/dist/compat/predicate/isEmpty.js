'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArguments = require('./isArguments.js');
const isArrayLike = require('./isArrayLike.js');
const isTypedArray = require('./isTypedArray.js');
const isPrototype = require('../_internal/isPrototype.js');

function isEmpty(value) {
    if (value == null) {
        return true;
    }
    if (isArrayLike.isArrayLike(value)) {
        if (typeof value.splice !== 'function' &&
            typeof value !== 'string' &&
            (typeof Buffer === 'undefined' || !Buffer.isBuffer(value)) &&
            !isTypedArray.isTypedArray(value) &&
            !isArguments.isArguments(value)) {
            return false;
        }
        return value.length === 0;
    }
    if (typeof value === 'object') {
        if (value instanceof Map || value instanceof Set) {
            return value.size === 0;
        }
        const keys = Object.keys(value);
        if (isPrototype.isPrototype(value)) {
            return keys.filter(x => x !== 'constructor').length === 0;
        }
        return keys.length === 0;
    }
    return true;
}

exports.isEmpty = isEmpty;
