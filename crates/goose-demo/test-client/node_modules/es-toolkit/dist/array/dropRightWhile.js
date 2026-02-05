'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function dropRightWhile(arr, canContinueDropping) {
    for (let i = arr.length - 1; i >= 0; i--) {
        if (!canContinueDropping(arr[i], i, arr)) {
            return arr.slice(0, i + 1);
        }
    }
    return [];
}

exports.dropRightWhile = dropRightWhile;
