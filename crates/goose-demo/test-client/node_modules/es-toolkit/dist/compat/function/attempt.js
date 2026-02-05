'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function attempt(func, ...args) {
    try {
        return func(...args);
    }
    catch (e) {
        return e instanceof Error ? e : new Error(e);
    }
}

exports.attempt = attempt;
