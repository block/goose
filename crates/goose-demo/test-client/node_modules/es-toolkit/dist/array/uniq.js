'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function uniq(arr) {
    return [...new Set(arr)];
}

exports.uniq = uniq;
