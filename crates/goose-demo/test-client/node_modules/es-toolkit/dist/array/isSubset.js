'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const difference = require('./difference.js');

function isSubset(superset, subset) {
    return difference.difference(subset, superset).length === 0;
}

exports.isSubset = isSubset;
