'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function reduce(map, callback, initialValue) {
    if (initialValue == null && map.size === 0) {
        throw new TypeError('Reduce of empty map with no initial value');
    }
    let accumulator = initialValue;
    for (const [key, value] of map) {
        if (accumulator == null) {
            accumulator = value;
        }
        else {
            accumulator = callback(accumulator, value, key, map);
        }
    }
    return accumulator;
}

exports.reduce = reduce;
