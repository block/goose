'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function some(map, doesMatch) {
    for (const [key, value] of map) {
        if (doesMatch(value, key, map)) {
            return true;
        }
    }
    return false;
}

exports.some = some;
