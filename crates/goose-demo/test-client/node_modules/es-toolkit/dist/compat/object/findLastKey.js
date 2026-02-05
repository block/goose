'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../function/identity.js');
const isObject = require('../predicate/isObject.js');
const iteratee = require('../util/iteratee.js');

function findLastKey(obj, predicate) {
    if (!isObject.isObject(obj)) {
        return undefined;
    }
    const iteratee$1 = iteratee.iteratee(predicate ?? identity.identity);
    const keys = Object.keys(obj);
    return keys.findLast(key => iteratee$1(obj[key], key, obj));
}

exports.findLastKey = findLastKey;
