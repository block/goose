'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isPlainObject = require('../predicate/isPlainObject.js');

function flattenObject(object, { delimiter = '.' } = {}) {
    return flattenObjectImpl(object, '', delimiter);
}
function flattenObjectImpl(object, prefix, delimiter) {
    const result = {};
    const keys = Object.keys(object);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        const prefixedKey = prefix ? `${prefix}${delimiter}${key}` : key;
        if (isPlainObject.isPlainObject(value) && Object.keys(value).length > 0) {
            Object.assign(result, flattenObjectImpl(value, prefixedKey, delimiter));
            continue;
        }
        if (Array.isArray(value) && value.length > 0) {
            Object.assign(result, flattenObjectImpl(value, prefixedKey, delimiter));
            continue;
        }
        result[prefixedKey] = value;
    }
    return result;
}

exports.flattenObject = flattenObject;
