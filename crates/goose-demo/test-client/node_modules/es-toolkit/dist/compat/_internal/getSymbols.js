'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function getSymbols(object) {
    return Object.getOwnPropertySymbols(object).filter(symbol => Object.prototype.propertyIsEnumerable.call(object, symbol));
}

exports.getSymbols = getSymbols;
