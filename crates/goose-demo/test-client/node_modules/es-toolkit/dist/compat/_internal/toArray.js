'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function toArray(value) {
    return Array.isArray(value) ? value : Array.from(value);
}

exports.toArray = toArray;
