'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sumBy = require('./sumBy.js');

function sum(array) {
    return sumBy.sumBy(array);
}

exports.sum = sum;
