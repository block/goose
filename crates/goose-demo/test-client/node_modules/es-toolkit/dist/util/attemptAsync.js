'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

async function attemptAsync(func) {
    try {
        const result = await func();
        return [null, result];
    }
    catch (error) {
        return [error, null];
    }
}

exports.attemptAsync = attemptAsync;
