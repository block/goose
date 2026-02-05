'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toNumber = require('../util/toNumber.js');
const toString = require('../util/toString.js');

function add(value, other) {
    if (value === undefined && other === undefined) {
        return 0;
    }
    if (value === undefined || other === undefined) {
        return value ?? other;
    }
    if (typeof value === 'string' || typeof other === 'string') {
        value = toString.toString(value);
        other = toString.toString(other);
    }
    else {
        value = toNumber.toNumber(value);
        other = toNumber.toNumber(other);
    }
    return value + other;
}

exports.add = add;
