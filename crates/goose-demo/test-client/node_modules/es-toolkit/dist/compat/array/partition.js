'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const iteratee = require('../util/iteratee.js');

function partition(source, predicate = identity.identity) {
    if (!source) {
        return [[], []];
    }
    const collection = isArrayLike.isArrayLike(source) ? source : Object.values(source);
    predicate = iteratee.iteratee(predicate);
    const matched = [];
    const unmatched = [];
    for (let i = 0; i < collection.length; i++) {
        const value = collection[i];
        if (predicate(value)) {
            matched.push(value);
        }
        else {
            unmatched.push(value);
        }
    }
    return [matched, unmatched];
}

exports.partition = partition;
