'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('../../array/flatten.js');

function concat(...values) {
    return flatten.flatten(values);
}

exports.concat = concat;
