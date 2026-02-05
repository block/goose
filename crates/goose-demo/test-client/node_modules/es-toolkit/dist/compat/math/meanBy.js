'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const meanBy$1 = require('../../math/meanBy.js');
const iteratee = require('../util/iteratee.js');

function meanBy(items, iteratee$1) {
    if (items == null) {
        return NaN;
    }
    return meanBy$1.meanBy(Array.from(items), iteratee.iteratee(iteratee$1 ?? identity.identity));
}

exports.meanBy = meanBy;
