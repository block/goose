'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const decimalAdjust = require('../_internal/decimalAdjust.js');

function ceil(number, precision = 0) {
    return decimalAdjust.decimalAdjust('ceil', number, precision);
}

exports.ceil = ceil;
