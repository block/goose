'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function mapValues(map, getNewValue) {
    const result = new Map();
    for (const [key, value] of map) {
        const newValue = getNewValue(value, key, map);
        result.set(key, newValue);
    }
    return result;
}

exports.mapValues = mapValues;
