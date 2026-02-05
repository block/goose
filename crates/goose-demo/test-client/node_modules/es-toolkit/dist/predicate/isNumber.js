'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isNumber(x) {
    return typeof x === 'number' || x instanceof Number;
}

exports.isNumber = isNumber;
