'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLike = require('../predicate/isArrayLike.js');

function fromPairs(pairs) {
    if (!isArrayLike.isArrayLike(pairs)) {
        return {};
    }
    const result = {};
    for (let i = 0; i < pairs.length; i++) {
        const [key, value] = pairs[i];
        result[key] = value;
    }
    return result;
}

exports.fromPairs = fromPairs;
