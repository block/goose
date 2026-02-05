'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isFunction = require('../../predicate/isFunction.js');

function functionsIn(object) {
    if (object == null) {
        return [];
    }
    const result = [];
    for (const key in object) {
        if (isFunction.isFunction(object[key])) {
            result.push(key);
        }
    }
    return result;
}

exports.functionsIn = functionsIn;
