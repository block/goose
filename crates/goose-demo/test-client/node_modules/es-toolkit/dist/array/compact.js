'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function compact(arr) {
    const result = [];
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        if (item) {
            result.push(item);
        }
    }
    return result;
}

exports.compact = compact;
