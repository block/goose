'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function fill(array, value, start = 0, end = array.length) {
    const length = array.length;
    const finalStart = Math.max(start >= 0 ? start : length + start, 0);
    const finalEnd = Math.min(end >= 0 ? end : length + end, length);
    for (let i = finalStart; i < finalEnd; i++) {
        array[i] = value;
    }
    return array;
}

exports.fill = fill;
