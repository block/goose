'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const iteratee = require('../util/iteratee.js');

function filter(source, predicate = identity.identity) {
    if (!source) {
        return [];
    }
    predicate = iteratee.iteratee(predicate);
    if (!Array.isArray(source)) {
        const result = [];
        const keys = Object.keys(source);
        const length = isArrayLike.isArrayLike(source) ? source.length : keys.length;
        for (let i = 0; i < length; i++) {
            const key = keys[i];
            const value = source[key];
            if (predicate(value, key, source)) {
                result.push(value);
            }
        }
        return result;
    }
    const result = [];
    const length = source.length;
    for (let i = 0; i < length; i++) {
        const value = source[i];
        if (predicate(value, i, source)) {
            result.push(value);
        }
    }
    return result;
}

exports.filter = filter;
