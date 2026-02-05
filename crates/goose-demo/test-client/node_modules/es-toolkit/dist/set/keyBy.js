'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function keyBy(set, getKeyFromValue) {
    const result = new Map();
    for (const value of set) {
        const newKey = getKeyFromValue(value, value, set);
        result.set(newKey, value);
    }
    return result;
}

exports.keyBy = keyBy;
