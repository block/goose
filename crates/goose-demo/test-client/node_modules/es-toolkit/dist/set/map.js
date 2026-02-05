'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function map(set, getNewValue) {
    const result = new Set();
    for (const value of set) {
        const newValue = getNewValue(value, value, set);
        result.add(newValue);
    }
    return result;
}

exports.map = map;
