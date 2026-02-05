'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isPromise(value) {
    return value instanceof Promise;
}

exports.isPromise = isPromise;
