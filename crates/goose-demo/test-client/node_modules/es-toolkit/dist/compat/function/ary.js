'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const ary$1 = require('../../function/ary.js');

function ary(func, n = func.length, guard) {
    if (guard) {
        n = func.length;
    }
    if (Number.isNaN(n) || n < 0) {
        n = 0;
    }
    return ary$1.ary(func, n);
}

exports.ary = ary;
