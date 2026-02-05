'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function inRange(value, minimum, maximum) {
    if (maximum == null) {
        maximum = minimum;
        minimum = 0;
    }
    if (minimum >= maximum) {
        throw new Error('The maximum value must be greater than the minimum value.');
    }
    return minimum <= value && value < maximum;
}

exports.inRange = inRange;
