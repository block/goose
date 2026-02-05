'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const decimalAdjust = require('../_internal/decimalAdjust.js');

function floor(number, precision = 0) {
    return decimalAdjust.decimalAdjust('floor', number, precision);
}

exports.floor = floor;
