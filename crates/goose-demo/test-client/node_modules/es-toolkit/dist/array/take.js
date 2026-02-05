'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toInteger = require('../compat/util/toInteger.js');

function take(arr, count, guard) {
    count = guard || count === undefined ? 1 : toInteger.toInteger(count);
    return arr.slice(0, count);
}

exports.take = take;
