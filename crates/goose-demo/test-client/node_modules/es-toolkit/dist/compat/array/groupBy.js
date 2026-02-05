'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const groupBy$1 = require('../../array/groupBy.js');
const identity = require('../../function/identity.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const iteratee = require('../util/iteratee.js');

function groupBy(source, _getKeyFromItem) {
    if (source == null) {
        return {};
    }
    const items = isArrayLike.isArrayLike(source) ? Array.from(source) : Object.values(source);
    const getKeyFromItem = iteratee.iteratee(_getKeyFromItem ?? identity.identity);
    return groupBy$1.groupBy(items, getKeyFromItem);
}

exports.groupBy = groupBy;
