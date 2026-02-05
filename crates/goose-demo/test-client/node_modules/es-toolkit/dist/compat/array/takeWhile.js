'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toArray = require('../_internal/toArray.js');
const identity = require('../function/identity.js');
const negate = require('../function/negate.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const iteratee = require('../util/iteratee.js');

function takeWhile(array, predicate) {
    if (!isArrayLikeObject.isArrayLikeObject(array)) {
        return [];
    }
    const _array = toArray.toArray(array);
    const index = _array.findIndex(negate.negate(iteratee.iteratee(predicate ?? identity.identity)));
    return index === -1 ? _array : _array.slice(0, index);
}

exports.takeWhile = takeWhile;
