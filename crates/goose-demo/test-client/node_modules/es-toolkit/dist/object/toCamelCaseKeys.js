'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArray = require('../compat/predicate/isArray.js');
const isPlainObject = require('../predicate/isPlainObject.js');
const camelCase = require('../string/camelCase.js');

function toCamelCaseKeys(obj) {
    if (isArray.isArray(obj)) {
        return obj.map(item => toCamelCaseKeys(item));
    }
    if (isPlainObject.isPlainObject(obj)) {
        const result = {};
        const keys = Object.keys(obj);
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            const camelKey = camelCase.camelCase(key);
            const convertedValue = toCamelCaseKeys(obj[key]);
            result[camelKey] = convertedValue;
        }
        return result;
    }
    return obj;
}

exports.toCamelCaseKeys = toCamelCaseKeys;
