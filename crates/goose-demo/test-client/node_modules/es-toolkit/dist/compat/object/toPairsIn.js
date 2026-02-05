'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keysIn = require('./keysIn.js');
const mapToEntries = require('../_internal/mapToEntries.js');
const setToEntries = require('../_internal/setToEntries.js');

function toPairsIn(object) {
    if (object == null) {
        return [];
    }
    if (object instanceof Set) {
        return setToEntries.setToEntries(object);
    }
    if (object instanceof Map) {
        return mapToEntries.mapToEntries(object);
    }
    const keys = keysIn.keysIn(object);
    const result = new Array(keys.length);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        result[i] = [key, value];
    }
    return result;
}

exports.toPairsIn = toPairsIn;
