'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function some(set, doesMatch) {
    for (const value of set) {
        if (doesMatch(value, value, set)) {
            return true;
        }
    }
    return false;
}

exports.some = some;
