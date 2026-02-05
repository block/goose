'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const isNil = require('../../predicate/isNil.js');
const iteratee = require('../util/iteratee.js');

function invertBy(object, iteratee$1) {
    const result = {};
    if (isNil.isNil(object)) {
        return result;
    }
    if (iteratee$1 == null) {
        iteratee$1 = identity.identity;
    }
    const keys = Object.keys(object);
    const getString = iteratee.iteratee(iteratee$1);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        const valueStr = getString(value);
        if (Array.isArray(result[valueStr])) {
            result[valueStr].push(key);
        }
        else {
            result[valueStr] = [key];
        }
    }
    return result;
}

exports.invertBy = invertBy;
