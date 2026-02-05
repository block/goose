'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function ary(func, n) {
    return function (...args) {
        return func.apply(this, args.slice(0, n));
    };
}

exports.ary = ary;
