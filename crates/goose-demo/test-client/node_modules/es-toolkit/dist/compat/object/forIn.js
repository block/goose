'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');

function forIn(object, iteratee = identity.identity) {
    if (object == null) {
        return object;
    }
    for (const key in object) {
        const result = iteratee(object[key], key, object);
        if (result === false) {
            break;
        }
    }
    return object;
}

exports.forIn = forIn;
