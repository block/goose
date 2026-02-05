'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const findKey$1 = require('../../object/findKey.js');
const identity = require('../function/identity.js');
const isObject = require('../predicate/isObject.js');
const iteratee = require('../util/iteratee.js');

function findKey(obj, predicate) {
    if (!isObject.isObject(obj)) {
        return undefined;
    }
    const iteratee$1 = iteratee.iteratee(predicate ?? identity.identity);
    return findKey$1.findKey(obj, iteratee$1);
}

exports.findKey = findKey;
