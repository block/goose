'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function invariant(condition, message) {
    if (condition) {
        return;
    }
    if (typeof message === 'string') {
        throw new Error(message);
    }
    throw message;
}

exports.invariant = invariant;
