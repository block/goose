'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function find(set, doesMatch) {
    for (const value of set) {
        if (doesMatch(value, value, set)) {
            return value;
        }
    }
    return undefined;
}

exports.find = find;
