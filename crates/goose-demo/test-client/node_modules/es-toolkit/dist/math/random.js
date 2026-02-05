'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function random(minimum, maximum) {
    if (maximum == null) {
        maximum = minimum;
        minimum = 0;
    }
    if (minimum >= maximum) {
        throw new Error('Invalid input: The maximum value must be greater than the minimum value.');
    }
    return Math.random() * (maximum - minimum) + minimum;
}

exports.random = random;
