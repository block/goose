'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keys = require('./keys.js');
const identity = require('../../function/identity.js');

function forOwnRight(object, iteratee = identity.identity) {
    if (object == null) {
        return object;
    }
    const iterable = Object(object);
    const keys$1 = keys.keys(object);
    for (let i = keys$1.length - 1; i >= 0; --i) {
        const key = keys$1[i];
        if (iteratee(iterable[key], key, iterable) === false) {
            break;
        }
    }
    return object;
}

exports.forOwnRight = forOwnRight;
