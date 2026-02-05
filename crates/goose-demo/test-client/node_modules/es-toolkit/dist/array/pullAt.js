'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const at = require('./at.js');

function pullAt(arr, indicesToRemove) {
    const removed = at.at(arr, indicesToRemove);
    const indices = new Set(indicesToRemove.slice().sort((x, y) => y - x));
    for (const index of indices) {
        arr.splice(index, 1);
    }
    return removed;
}

exports.pullAt = pullAt;
