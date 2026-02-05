'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const iteratee = require('../util/iteratee.js');

function pullAllBy(arr, valuesToRemove, _getValue) {
    const getValue = iteratee.iteratee(_getValue);
    const valuesSet = new Set(Array.from(valuesToRemove).map(x => getValue(x)));
    let resultIndex = 0;
    for (let i = 0; i < arr.length; i++) {
        const value = getValue(arr[i]);
        if (valuesSet.has(value)) {
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

exports.pullAllBy = pullAllBy;
