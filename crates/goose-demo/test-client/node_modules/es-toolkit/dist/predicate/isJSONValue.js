'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isPlainObject = require('./isPlainObject.js');

function isJSONValue(value) {
    switch (typeof value) {
        case 'object': {
            return value === null || isJSONArray(value) || isJSONObject(value);
        }
        case 'string':
        case 'number':
        case 'boolean': {
            return true;
        }
        default: {
            return false;
        }
    }
}
function isJSONArray(value) {
    if (!Array.isArray(value)) {
        return false;
    }
    return value.every(item => isJSONValue(item));
}
function isJSONObject(obj) {
    if (!isPlainObject.isPlainObject(obj)) {
        return false;
    }
    const keys = Reflect.ownKeys(obj);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = obj[key];
        if (typeof key !== 'string') {
            return false;
        }
        if (!isJSONValue(value)) {
            return false;
        }
    }
    return true;
}

exports.isJSONArray = isJSONArray;
exports.isJSONObject = isJSONObject;
exports.isJSONValue = isJSONValue;
