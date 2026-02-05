'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isPrimitive(value) {
    return value == null || (typeof value !== 'object' && typeof value !== 'function');
}

exports.isPrimitive = isPrimitive;
