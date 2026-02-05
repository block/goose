'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isLength(value) {
    return Number.isSafeInteger(value) && value >= 0;
}

exports.isLength = isLength;
