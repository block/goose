'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const maxBy$1 = require('../../array/maxBy.js');
const identity = require('../../function/identity.js');
const iteratee = require('../util/iteratee.js');

function maxBy(items, iteratee$1) {
    if (items == null) {
        return undefined;
    }
    return maxBy$1.maxBy(Array.from(items), iteratee.iteratee(iteratee$1 ?? identity.identity));
}

exports.maxBy = maxBy;
