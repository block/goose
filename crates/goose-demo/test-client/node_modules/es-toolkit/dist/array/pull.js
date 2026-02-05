'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function pull(arr, valuesToRemove) {
    const valuesSet = new Set(valuesToRemove);
    let resultIndex = 0;
    for (let i = 0; i < arr.length; i++) {
        if (valuesSet.has(arr[i])) {
            continue;
        }
        if (!Object.hasOwn(arr, i)) {
            delete arr[resultIndex++];
            continue;
        }
        arr[resultIndex++] = arr[i];
    }
    arr.length = resultIndex;
    return arr;
}

exports.pull = pull;
