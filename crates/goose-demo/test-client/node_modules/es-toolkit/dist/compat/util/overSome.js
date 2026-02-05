'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const iteratee = require('./iteratee.js');

function overSome(...predicates) {
    return function (...values) {
        for (let i = 0; i < predicates.length; ++i) {
            const predicate = predicates[i];
            if (!Array.isArray(predicate)) {
                if (iteratee.iteratee(predicate).apply(this, values)) {
                    return true;
                }
                continue;
            }
            for (let j = 0; j < predicate.length; ++j) {
                if (iteratee.iteratee(predicate[j]).apply(this, values)) {
                    return true;
                }
            }
        }
        return false;
    };
}

exports.overSome = overSome;
