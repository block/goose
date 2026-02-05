'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toInteger = require('../util/toInteger.js');

function nthArg(n = 0) {
    return function (...args) {
        return args.at(toInteger.toInteger(n));
    };
}

exports.nthArg = nthArg;
