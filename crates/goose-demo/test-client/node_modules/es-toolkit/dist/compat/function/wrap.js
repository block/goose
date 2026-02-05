'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const isFunction = require('../../predicate/isFunction.js');

function wrap(value, wrapper) {
    return function (...args) {
        const wrapFn = isFunction.isFunction(wrapper) ? wrapper : identity.identity;
        return wrapFn.apply(this, [value, ...args]);
    };
}

exports.wrap = wrap;
