'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keysIn = require('../object/keysIn.js');

function toPlainObject(value) {
    const plainObject = {};
    const valueKeys = keysIn.keysIn(value);
    for (let i = 0; i < valueKeys.length; i++) {
        const key = valueKeys[i];
        const objValue = value[key];
        if (key === '__proto__') {
            Object.defineProperty(plainObject, key, {
                configurable: true,
                enumerable: true,
                value: objValue,
                writable: true,
            });
        }
        else {
            plainObject[key] = objValue;
        }
    }
    return plainObject;
}

exports.toPlainObject = toPlainObject;
