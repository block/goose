'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isSymbol(value) {
    return typeof value === 'symbol' || value instanceof Symbol;
}

exports.isSymbol = isSymbol;
