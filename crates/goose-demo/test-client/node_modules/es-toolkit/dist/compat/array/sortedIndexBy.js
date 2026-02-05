'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isNull = require('../../predicate/isNull.js');
const isUndefined = require('../../predicate/isUndefined.js');
const identity = require('../function/identity.js');
const isNaN = require('../predicate/isNaN.js');
const isNil = require('../predicate/isNil.js');
const isSymbol = require('../predicate/isSymbol.js');
const iteratee = require('../util/iteratee.js');

const MAX_ARRAY_LENGTH = 4294967295;
const MAX_ARRAY_INDEX = MAX_ARRAY_LENGTH - 1;
function sortedIndexBy(array, value, iteratee$1 = identity.identity, retHighest) {
    if (isNil.isNil(array) || array.length === 0) {
        return 0;
    }
    let low = 0;
    let high = array.length;
    const iterateeFunction = iteratee.iteratee(iteratee$1);
    const transformedValue = iterateeFunction(value);
    const valIsNaN = isNaN.isNaN(transformedValue);
    const valIsNull = isNull.isNull(transformedValue);
    const valIsSymbol = isSymbol.isSymbol(transformedValue);
    const valIsUndefined = isUndefined.isUndefined(transformedValue);
    while (low < high) {
        let setLow;
        const mid = Math.floor((low + high) / 2);
        const computed = iterateeFunction(array[mid]);
        const othIsDefined = !isUndefined.isUndefined(computed);
        const othIsNull = isNull.isNull(computed);
        const othIsReflexive = !isNaN.isNaN(computed);
        const othIsSymbol = isSymbol.isSymbol(computed);
        if (valIsNaN) {
            setLow = retHighest || othIsReflexive;
        }
        else if (valIsUndefined) {
            setLow = othIsReflexive && (retHighest || othIsDefined);
        }
        else if (valIsNull) {
            setLow = othIsReflexive && othIsDefined && (retHighest || !othIsNull);
        }
        else if (valIsSymbol) {
            setLow = othIsReflexive && othIsDefined && !othIsNull && (retHighest || !othIsSymbol);
        }
        else if (othIsNull || othIsSymbol) {
            setLow = false;
        }
        else {
            setLow = retHighest ? computed <= transformedValue : computed < transformedValue;
        }
        if (setLow) {
            low = mid + 1;
        }
        else {
            high = mid;
        }
    }
    return Math.min(high, MAX_ARRAY_INDEX);
}

exports.sortedIndexBy = sortedIndexBy;
