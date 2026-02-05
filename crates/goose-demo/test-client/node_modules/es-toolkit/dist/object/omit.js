'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function omit(obj, keys) {
    const result = { ...obj };
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        delete result[key];
    }
    return result;
}

exports.omit = omit;
