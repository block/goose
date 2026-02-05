'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function mapKeys(map, getNewKey) {
    const result = new Map();
    for (const [key, value] of map) {
        const newKey = getNewKey(value, key, map);
        result.set(newKey, value);
    }
    return result;
}

exports.mapKeys = mapKeys;
