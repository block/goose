'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function countBy(arr, mapper) {
    const result = {};
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        const key = mapper(item, i, arr);
        result[key] = (result[key] ?? 0) + 1;
    }
    return result;
}

exports.countBy = countBy;
