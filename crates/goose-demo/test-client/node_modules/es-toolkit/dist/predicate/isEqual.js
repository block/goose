'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isEqualWith = require('./isEqualWith.js');
const noop = require('../function/noop.js');

function isEqual(a, b) {
    return isEqualWith.isEqualWith(a, b, noop.noop);
}

exports.isEqual = isEqual;
