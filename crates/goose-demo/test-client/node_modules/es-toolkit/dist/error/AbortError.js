'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

class AbortError extends Error {
    constructor(message = 'The operation was aborted') {
        super(message);
        this.name = 'AbortError';
    }
}

exports.AbortError = AbortError;
