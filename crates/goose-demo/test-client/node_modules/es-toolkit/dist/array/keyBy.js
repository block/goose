'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function keyBy(arr, getKeyFromItem) {
    const result = {};
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        const key = getKeyFromItem(item, i, arr);
        result[key] = item;
    }
    return result;
}

exports.keyBy = keyBy;
