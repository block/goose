'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function zipWith(arr1, ...rest) {
    const arrs = [arr1, ...rest.slice(0, -1)];
    const combine = rest[rest.length - 1];
    const maxIndex = Math.max(...arrs.map(arr => arr.length));
    const result = Array(maxIndex);
    for (let i = 0; i < maxIndex; i++) {
        const elements = arrs.map(arr => arr[i]);
        result[i] = combine(...elements, i);
    }
    return result;
}

exports.zipWith = zipWith;
