'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

async function reduceAsync(array, reducer, initialValue) {
    let startIndex = 0;
    if (initialValue == null) {
        initialValue = array[0];
        startIndex = 1;
    }
    let accumulator = initialValue;
    for (let i = startIndex; i < array.length; i++) {
        accumulator = await reducer(accumulator, array[i], i, array);
    }
    return accumulator;
}

exports.reduceAsync = reduceAsync;
