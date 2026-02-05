'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function decimalAdjust(type, number, precision = 0) {
    number = Number(number);
    if (Object.is(number, -0)) {
        number = '-0';
    }
    precision = Math.min(Number.parseInt(precision, 10), 292);
    if (precision) {
        const [magnitude, exponent = 0] = number.toString().split('e');
        let adjustedValue = Math[type](Number(`${magnitude}e${Number(exponent) + precision}`));
        if (Object.is(adjustedValue, -0)) {
            adjustedValue = '-0';
        }
        const [newMagnitude, newExponent = 0] = adjustedValue.toString().split('e');
        return Number(`${newMagnitude}e${Number(newExponent) - precision}`);
    }
    return Math[type](Number(number));
}

exports.decimalAdjust = decimalAdjust;
