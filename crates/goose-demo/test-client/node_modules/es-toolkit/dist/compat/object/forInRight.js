'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');

function forInRight(object, iteratee = identity.identity) {
    if (object == null) {
        return object;
    }
    const keys = [];
    for (const key in object) {
        keys.push(key);
    }
    for (let i = keys.length - 1; i >= 0; i--) {
        const key = keys[i];
        const result = iteratee(object[key], key, object);
        if (result === false) {
            break;
        }
    }
    return object;
}

exports.forInRight = forInRight;
