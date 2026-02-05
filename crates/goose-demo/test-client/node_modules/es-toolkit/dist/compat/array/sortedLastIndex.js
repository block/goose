'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sortedLastIndexBy = require('./sortedLastIndexBy.js');
const isNil = require('../../predicate/isNil.js');
const isNull = require('../../predicate/isNull.js');
const isSymbol = require('../../predicate/isSymbol.js');
const isNumber = require('../predicate/isNumber.js');

const MAX_ARRAY_LENGTH = 4294967295;
const HALF_MAX_ARRAY_LENGTH = MAX_ARRAY_LENGTH >>> 1;
function sortedLastIndex(array, value) {
    if (isNil.isNil(array)) {
        return 0;
    }
    let high = array.length;
    if (!isNumber.isNumber(value) || Number.isNaN(value) || high > HALF_MAX_ARRAY_LENGTH) {
        return sortedLastIndexBy.sortedLastIndexBy(array, value, value => value);
    }
    let low = 0;
    while (low < high) {
        const mid = (low + high) >>> 1;
        const compute = array[mid];
        if (!isNull.isNull(compute) && !isSymbol.isSymbol(compute) && compute <= value) {
            low = mid + 1;
        }
        else {
            high = mid;
        }
    }
    return high;
}

exports.sortedLastIndex = sortedLastIndex;
