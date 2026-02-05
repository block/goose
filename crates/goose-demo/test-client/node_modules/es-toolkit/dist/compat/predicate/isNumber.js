'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isNumber(value) {
    return typeof value === 'number' || value instanceof Number;
}

exports.isNumber = isNumber;
