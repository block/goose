'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const MAX_ARRAY_LENGTH = require('../_internal/MAX_ARRAY_LENGTH.js');
const clamp = require('../math/clamp.js');

function toLength(value) {
    if (value == null) {
        return 0;
    }
    const length = Math.floor(Number(value));
    return clamp.clamp(length, 0, MAX_ARRAY_LENGTH.MAX_ARRAY_LENGTH);
}

exports.toLength = toLength;
