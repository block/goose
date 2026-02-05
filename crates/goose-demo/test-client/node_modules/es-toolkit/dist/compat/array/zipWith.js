'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const unzip = require('./unzip.js');
const isFunction = require('../../predicate/isFunction.js');

function zipWith(...combine) {
    let iteratee = combine.pop();
    if (!isFunction.isFunction(iteratee)) {
        combine.push(iteratee);
        iteratee = undefined;
    }
    if (!combine?.length) {
        return [];
    }
    const result = unzip.unzip(combine);
    if (iteratee == null) {
        return result;
    }
    return result.map(group => iteratee(...group));
}

exports.zipWith = zipWith;
