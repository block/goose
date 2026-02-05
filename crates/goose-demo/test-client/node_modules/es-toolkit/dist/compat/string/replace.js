'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toString = require('../util/toString.js');

function replace(target, pattern, replacement) {
    if (arguments.length < 3) {
        return toString.toString(target);
    }
    return toString.toString(target).replace(pattern, replacement);
}

exports.replace = replace;
