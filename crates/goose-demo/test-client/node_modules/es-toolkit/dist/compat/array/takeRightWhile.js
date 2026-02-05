'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const negate = require('../../function/negate.js');
const toArray = require('../_internal/toArray.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const iteratee = require('../util/iteratee.js');

function takeRightWhile(_array, predicate) {
    if (!isArrayLikeObject.isArrayLikeObject(_array)) {
        return [];
    }
    const array = toArray.toArray(_array);
    const index = array.findLastIndex(negate.negate(iteratee.iteratee(predicate ?? identity.identity)));
    return array.slice(index + 1);
}

exports.takeRightWhile = takeRightWhile;
