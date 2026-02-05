'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const rest$1 = require('../../function/rest.js');

function rest(func, start = func.length - 1) {
    start = Number.parseInt(start, 10);
    if (Number.isNaN(start) || start < 0) {
        start = func.length - 1;
    }
    return rest$1.rest(func, start);
}

exports.rest = rest;
