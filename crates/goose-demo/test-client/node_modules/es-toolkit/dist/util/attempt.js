'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function attempt(func) {
    try {
        return [null, func()];
    }
    catch (error) {
        return [error, null];
    }
}

exports.attempt = attempt;
