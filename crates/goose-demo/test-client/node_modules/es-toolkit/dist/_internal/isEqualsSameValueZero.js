'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isEqualsSameValueZero(value, other) {
    return value === other || (Number.isNaN(value) && Number.isNaN(other));
}

exports.isEqualsSameValueZero = isEqualsSameValueZero;
