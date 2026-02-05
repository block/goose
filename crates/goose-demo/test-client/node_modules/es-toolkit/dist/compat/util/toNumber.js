'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isSymbol = require('../predicate/isSymbol.js');

function toNumber(value) {
    if (isSymbol.isSymbol(value)) {
        return NaN;
    }
    return Number(value);
}

exports.toNumber = toNumber;
