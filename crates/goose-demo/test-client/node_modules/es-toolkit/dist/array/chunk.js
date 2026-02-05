'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function chunk(arr, size) {
    if (!Number.isInteger(size) || size <= 0) {
        throw new Error('Size must be an integer greater than zero.');
    }
    const chunkLength = Math.ceil(arr.length / size);
    const result = Array(chunkLength);
    for (let index = 0; index < chunkLength; index++) {
        const start = index * size;
        const end = start + size;
        result[index] = arr.slice(start, end);
    }
    return result;
}

exports.chunk = chunk;
