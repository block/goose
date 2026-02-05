'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const decimalAdjust = require('../_internal/decimalAdjust.js');

function round(number, precision = 0) {
    return decimalAdjust.decimalAdjust('round', number, precision);
}

exports.round = round;
