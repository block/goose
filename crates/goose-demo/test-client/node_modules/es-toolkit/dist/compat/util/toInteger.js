'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toFinite = require('./toFinite.js');

function toInteger(value) {
    const finite = toFinite.toFinite(value);
    const remainder = finite % 1;
    return remainder ? finite - remainder : finite;
}

exports.toInteger = toInteger;
