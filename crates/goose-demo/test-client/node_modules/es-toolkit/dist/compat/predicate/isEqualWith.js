'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const after = require('../../function/after.js');
const isEqualWith$1 = require('../../predicate/isEqualWith.js');

function isEqualWith(a, b, areValuesEqual) {
    if (typeof areValuesEqual !== 'function') {
        areValuesEqual = () => undefined;
    }
    return isEqualWith$1.isEqualWith(a, b, (...args) => {
        const result = areValuesEqual(...args);
        if (result !== undefined) {
            return Boolean(result);
        }
        if (a instanceof Map && b instanceof Map) {
            return isEqualWith(Array.from(a), Array.from(b), after.after(2, areValuesEqual));
        }
        if (a instanceof Set && b instanceof Set) {
            return isEqualWith(Array.from(a), Array.from(b), after.after(2, areValuesEqual));
        }
    });
}

exports.isEqualWith = isEqualWith;
