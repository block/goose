'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keys = require('./keys.js');
const mapToEntries = require('../_internal/mapToEntries.js');
const setToEntries = require('../_internal/setToEntries.js');

function toPairs(object) {
    if (object == null) {
        return [];
    }
    if (object instanceof Set) {
        return setToEntries.setToEntries(object);
    }
    if (object instanceof Map) {
        return mapToEntries.mapToEntries(object);
    }
    const keys$1 = keys.keys(object);
    const result = new Array(keys$1.length);
    for (let i = 0; i < keys$1.length; i++) {
        const key = keys$1[i];
        const value = object[key];
        result[i] = [key, value];
    }
    return result;
}

exports.toPairs = toPairs;
