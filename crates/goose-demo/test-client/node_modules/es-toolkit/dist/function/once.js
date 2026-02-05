'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function once(func) {
    let called = false;
    let cache;
    return function (...args) {
        if (!called) {
            called = true;
            cache = func(...args);
        }
        return cache;
    };
}

exports.once = once;
