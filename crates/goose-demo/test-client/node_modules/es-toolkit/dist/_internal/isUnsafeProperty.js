'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isUnsafeProperty(key) {
    return key === '__proto__';
}

exports.isUnsafeProperty = isUnsafeProperty;
