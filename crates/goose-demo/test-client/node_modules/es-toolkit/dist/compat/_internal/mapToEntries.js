'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function mapToEntries(map) {
    const arr = new Array(map.size);
    const keys = map.keys();
    const values = map.values();
    for (let i = 0; i < arr.length; i++) {
        arr[i] = [keys.next().value, values.next().value];
    }
    return arr;
}

exports.mapToEntries = mapToEntries;
