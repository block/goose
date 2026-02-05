'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const reduce = require('./reduce.js');
const identity = require('../../function/identity.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isObjectLike = require('../predicate/isObjectLike.js');
const iteratee = require('../util/iteratee.js');

function keyBy(collection, iteratee$1) {
    if (!isArrayLike.isArrayLike(collection) && !isObjectLike.isObjectLike(collection)) {
        return {};
    }
    const keyFn = iteratee.iteratee(iteratee$1 ?? identity.identity);
    return reduce.reduce(collection, (result, value) => {
        const key = keyFn(value);
        result[key] = value;
        return result;
    }, {});
}

exports.keyBy = keyBy;
