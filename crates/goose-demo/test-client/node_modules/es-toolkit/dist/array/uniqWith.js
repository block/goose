'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function uniqWith(arr, areItemsEqual) {
    const result = [];
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        const isUniq = result.every(v => !areItemsEqual(v, item));
        if (isUniq) {
            result.push(item);
        }
    }
    return result;
}

exports.uniqWith = uniqWith;
