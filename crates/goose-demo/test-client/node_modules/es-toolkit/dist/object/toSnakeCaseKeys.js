'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArray = require('../compat/predicate/isArray.js');
const isPlainObject = require('../compat/predicate/isPlainObject.js');
const snakeCase = require('../string/snakeCase.js');

function toSnakeCaseKeys(obj) {
    if (isArray.isArray(obj)) {
        return obj.map(item => toSnakeCaseKeys(item));
    }
    if (isPlainObject.isPlainObject(obj)) {
        const result = {};
        const keys = Object.keys(obj);
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            const snakeKey = snakeCase.snakeCase(key);
            const convertedValue = toSnakeCaseKeys(obj[key]);
            result[snakeKey] = convertedValue;
        }
        return result;
    }
    return obj;
}

exports.toSnakeCaseKeys = toSnakeCaseKeys;
