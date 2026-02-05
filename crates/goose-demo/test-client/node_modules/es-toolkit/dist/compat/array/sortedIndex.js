'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sortedIndexBy = require('./sortedIndexBy.js');
const isNil = require('../../predicate/isNil.js');
const isNull = require('../../predicate/isNull.js');
const isSymbol = require('../../predicate/isSymbol.js');
const isNumber = require('../predicate/isNumber.js');

const MAX_ARRAY_LENGTH = 4294967295;
const HALF_MAX_ARRAY_LENGTH = MAX_ARRAY_LENGTH >>> 1;
function sortedIndex(array, value) {
    if (isNil.isNil(array)) {
        return 0;
    }
    let low = 0;
    let high = array.length;
    if (isNumber.isNumber(value) && value === value && high <= HALF_MAX_ARRAY_LENGTH) {
        while (low < high) {
            const mid = (low + high) >>> 1;
            const compute = array[mid];
            if (!isNull.isNull(compute) && !isSymbol.isSymbol(compute) && compute < value) {
                low = mid + 1;
            }
            else {
                high = mid;
            }
        }
        return high;
    }
    return sortedIndexBy.sortedIndexBy(array, value, value => value);
}

exports.sortedIndex = sortedIndex;
