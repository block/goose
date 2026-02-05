'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function forEachRight(arr, callback) {
    for (let i = arr.length - 1; i >= 0; i--) {
        const element = arr[i];
        callback(element, i, arr);
    }
}

exports.forEachRight = forEachRight;
