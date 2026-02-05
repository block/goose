'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isNil = require('../../predicate/isNil.js');

function size(target) {
    if (isNil.isNil(target)) {
        return 0;
    }
    if (target instanceof Map || target instanceof Set) {
        return target.size;
    }
    return Object.keys(target).length;
}

exports.size = size;
