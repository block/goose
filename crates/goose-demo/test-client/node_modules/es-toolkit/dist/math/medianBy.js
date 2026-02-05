'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const median = require('./median.js');

function medianBy(items, getValue) {
    const nums = items.map(x => getValue(x));
    return median.median(nums);
}

exports.medianBy = medianBy;
