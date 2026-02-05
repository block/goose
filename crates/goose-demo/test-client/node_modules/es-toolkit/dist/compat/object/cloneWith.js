'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const clone = require('./clone.js');

function cloneWith(value, customizer) {
    if (!customizer) {
        return clone.clone(value);
    }
    const result = customizer(value);
    if (result !== undefined) {
        return result;
    }
    return clone.clone(value);
}

exports.cloneWith = cloneWith;
