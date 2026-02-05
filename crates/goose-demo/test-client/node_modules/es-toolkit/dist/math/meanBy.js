'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sumBy = require('./sumBy.js');

function meanBy(items, getValue) {
    return sumBy.sumBy(items, item => getValue(item)) / items.length;
}

exports.meanBy = meanBy;
