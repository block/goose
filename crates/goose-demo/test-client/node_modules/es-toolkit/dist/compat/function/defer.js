'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function defer(func, ...args) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    return setTimeout(func, 1, ...args);
}

exports.defer = defer;
