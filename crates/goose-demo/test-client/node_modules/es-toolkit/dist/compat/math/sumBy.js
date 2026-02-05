'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const iteratee = require('../util/iteratee.js');

function sumBy(array, iteratee$1) {
    if (!array || !array.length) {
        return 0;
    }
    if (iteratee$1 != null) {
        iteratee$1 = iteratee.iteratee(iteratee$1);
    }
    let result = undefined;
    for (let i = 0; i < array.length; i++) {
        const current = iteratee$1 ? iteratee$1(array[i]) : array[i];
        if (current !== undefined) {
            if (result === undefined) {
                result = current;
            }
            else {
                result += current;
            }
        }
    }
    return result;
}

exports.sumBy = sumBy;
