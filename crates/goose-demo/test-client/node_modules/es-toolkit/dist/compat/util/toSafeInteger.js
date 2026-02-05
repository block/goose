'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toInteger = require('./toInteger.js');
const MAX_SAFE_INTEGER = require('../_internal/MAX_SAFE_INTEGER.js');
const clamp = require('../math/clamp.js');

function toSafeInteger(value) {
    if (value == null) {
        return 0;
    }
    return clamp.clamp(toInteger.toInteger(value), -MAX_SAFE_INTEGER.MAX_SAFE_INTEGER, MAX_SAFE_INTEGER.MAX_SAFE_INTEGER);
}

exports.toSafeInteger = toSafeInteger;
