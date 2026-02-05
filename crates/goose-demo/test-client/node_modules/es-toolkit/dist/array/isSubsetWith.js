'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const differenceWith = require('./differenceWith.js');

function isSubsetWith(superset, subset, areItemsEqual) {
    return differenceWith.differenceWith(subset, superset, areItemsEqual).length === 0;
}

exports.isSubsetWith = isSubsetWith;
