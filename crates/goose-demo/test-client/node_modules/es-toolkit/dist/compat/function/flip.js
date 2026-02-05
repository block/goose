'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function flip(func) {
    return function (...args) {
        return func.apply(this, args.reverse());
    };
}

exports.flip = flip;
