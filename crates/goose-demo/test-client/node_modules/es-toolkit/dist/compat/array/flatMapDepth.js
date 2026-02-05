'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('./flatten.js');
const map = require('./map.js');
const identity = require('../../function/identity.js');
const iteratee = require('../util/iteratee.js');

function flatMapDepth(collection, iteratee$1 = identity.identity, depth = 1) {
    if (collection == null) {
        return [];
    }
    const iterateeFn = iteratee.iteratee(iteratee$1);
    const mapped = map.map(collection, iterateeFn);
    return flatten.flatten(mapped, depth);
}

exports.flatMapDepth = flatMapDepth;
