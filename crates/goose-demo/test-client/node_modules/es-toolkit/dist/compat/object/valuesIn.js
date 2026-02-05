'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keysIn = require('./keysIn.js');

function valuesIn(object) {
    const keys = keysIn.keysIn(object);
    const result = new Array(keys.length);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        result[i] = object[key];
    }
    return result;
}

exports.valuesIn = valuesIn;
