'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function toKey(value) {
    if (typeof value === 'string' || typeof value === 'symbol') {
        return value;
    }
    if (Object.is(value?.valueOf?.(), -0)) {
        return '-0';
    }
    return String(value);
}

exports.toKey = toKey;
