'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function every(set, doesMatch) {
    for (const value of set) {
        if (!doesMatch(value, value, set)) {
            return false;
        }
    }
    return true;
}

exports.every = every;
