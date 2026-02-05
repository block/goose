'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function defaultTo(value, defaultValue) {
    if (value == null || Number.isNaN(value)) {
        return defaultValue;
    }
    return value;
}

exports.defaultTo = defaultTo;
