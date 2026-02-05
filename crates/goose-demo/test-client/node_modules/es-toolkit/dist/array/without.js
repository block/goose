'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const difference = require('./difference.js');

function without(array, ...values) {
    return difference.difference(array, values);
}

exports.without = without;
