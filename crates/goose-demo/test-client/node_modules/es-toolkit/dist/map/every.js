'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function every(map, doesMatch) {
    for (const [key, value] of map) {
        if (!doesMatch(value, key, map)) {
            return false;
        }
    }
    return true;
}

exports.every = every;
