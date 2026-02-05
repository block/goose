'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function findValue(map, doesMatch) {
    let result = undefined;
    for (const [key, value] of map) {
        if (doesMatch(value, key, map)) {
            result = value;
            break;
        }
    }
    return result;
}

exports.findValue = findValue;
