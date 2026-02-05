'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isString = require('../predicate/isString.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');
const toInteger = require('../util/toInteger.js');

function includes(source, target, fromIndex, guard) {
    if (source == null) {
        return false;
    }
    if (guard || !fromIndex) {
        fromIndex = 0;
    }
    else {
        fromIndex = toInteger.toInteger(fromIndex);
    }
    if (isString.isString(source)) {
        if (fromIndex > source.length || target instanceof RegExp) {
            return false;
        }
        if (fromIndex < 0) {
            fromIndex = Math.max(0, source.length + fromIndex);
        }
        return source.includes(target, fromIndex);
    }
    if (Array.isArray(source)) {
        return source.includes(target, fromIndex);
    }
    const keys = Object.keys(source);
    if (fromIndex < 0) {
        fromIndex = Math.max(0, keys.length + fromIndex);
    }
    for (let i = fromIndex; i < keys.length; i++) {
        const value = Reflect.get(source, keys[i]);
        if (isEqualsSameValueZero.isEqualsSameValueZero(value, target)) {
            return true;
        }
    }
    return false;
}

exports.includes = includes;
