'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function mapValues(object, getNewValue) {
    const result = {};
    const keys = Object.keys(object);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        result[key] = getNewValue(value, key, object);
    }
    return result;
}

exports.mapValues = mapValues;
