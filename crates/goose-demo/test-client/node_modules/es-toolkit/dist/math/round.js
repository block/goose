'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function round(value, precision = 0) {
    if (!Number.isInteger(precision)) {
        throw new Error('Precision must be an integer.');
    }
    const multiplier = Math.pow(10, precision);
    return Math.round(value * multiplier) / multiplier;
}

exports.round = round;
