'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const minBy$1 = require('../../array/minBy.js');
const identity = require('../../function/identity.js');
const iteratee = require('../util/iteratee.js');

function minBy(items, iteratee$1) {
    if (items == null) {
        return undefined;
    }
    return minBy$1.minBy(Array.from(items), iteratee.iteratee(iteratee$1 ?? identity.identity));
}

exports.minBy = minBy;
