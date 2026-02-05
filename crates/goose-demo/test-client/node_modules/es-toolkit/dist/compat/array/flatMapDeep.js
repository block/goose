'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatMapDepth = require('./flatMapDepth.js');

function flatMapDeep(collection, iteratee) {
    return flatMapDepth.flatMapDepth(collection, iteratee, Infinity);
}

exports.flatMapDeep = flatMapDeep;
