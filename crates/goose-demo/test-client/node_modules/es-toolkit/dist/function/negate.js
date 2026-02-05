'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function negate(func) {
    return ((...args) => !func(...args));
}

exports.negate = negate;
