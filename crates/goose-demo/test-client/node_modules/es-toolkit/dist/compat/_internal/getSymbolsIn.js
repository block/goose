'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const getSymbols = require('./getSymbols.js');

function getSymbolsIn(object) {
    const result = [];
    while (object) {
        result.push(...getSymbols.getSymbols(object));
        object = Object.getPrototypeOf(object);
    }
    return result;
}

exports.getSymbolsIn = getSymbolsIn;
