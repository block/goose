'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toNumber = require('./toNumber.js');

function toFinite(value) {
    if (!value) {
        return value === 0 ? value : 0;
    }
    value = toNumber.toNumber(value);
    if (value === Infinity || value === -Infinity) {
        const sign = value < 0 ? -1 : 1;
        return sign * Number.MAX_VALUE;
    }
    return value === value ? value : 0;
}

exports.toFinite = toFinite;
