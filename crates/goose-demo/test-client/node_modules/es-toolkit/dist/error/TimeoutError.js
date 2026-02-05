'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

class TimeoutError extends Error {
    constructor(message = 'The operation was timed out') {
        super(message);
        this.name = 'TimeoutError';
    }
}

exports.TimeoutError = TimeoutError;
