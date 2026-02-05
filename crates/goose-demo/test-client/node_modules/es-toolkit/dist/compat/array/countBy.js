'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('../predicate/isArrayLike.js');
const iteratee = require('../util/iteratee.js');

function countBy(collection, iteratee$1) {
    if (collection == null) {
        return {};
    }
    const array = isArrayLike.isArrayLike(collection) ? Array.from(collection) : Object.values(collection);
    const mapper = iteratee.iteratee(iteratee$1 ?? undefined);
    const result = Object.create(null);
    for (let i = 0; i < array.length; i++) {
        const item = array[i];
        const key = mapper(item);
        result[key] = (result[key] ?? 0) + 1;
    }
    return result;
}

exports.countBy = countBy;
