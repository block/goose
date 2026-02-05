'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toNumber = require('./toNumber.js');

function lte(value, other) {
    if (typeof value === 'string' && typeof other === 'string') {
        return value <= other;
    }
    return toNumber.toNumber(value) <= toNumber.toNumber(other);
}

exports.lte = lte;
