'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const iteratee = require('./iteratee.js');
const isFunction = require('../../predicate/isFunction.js');

function cond(pairs) {
    const length = pairs.length;
    const processedPairs = pairs.map(pair => {
        const predicate = pair[0];
        const func = pair[1];
        if (!isFunction.isFunction(func)) {
            throw new TypeError('Expected a function');
        }
        return [iteratee.iteratee(predicate), func];
    });
    return function (...args) {
        for (let i = 0; i < length; i++) {
            const pair = processedPairs[i];
            const predicate = pair[0];
            const func = pair[1];
            if (predicate.apply(this, args)) {
                return func.apply(this, args);
            }
        }
    };
}

exports.cond = cond;
