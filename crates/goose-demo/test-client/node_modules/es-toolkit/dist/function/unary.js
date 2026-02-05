'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const ary = require('./ary.js');

function unary(func) {
    return ary.ary(func, 1);
}

exports.unary = unary;
