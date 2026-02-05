'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const orderBy = require('./orderBy.js');
const flatten = require('../../array/flatten.js');
const isIterateeCall = require('../_internal/isIterateeCall.js');

function sortBy(collection, ...criteria) {
    const length = criteria.length;
    if (length > 1 && isIterateeCall.isIterateeCall(collection, criteria[0], criteria[1])) {
        criteria = [];
    }
    else if (length > 2 && isIterateeCall.isIterateeCall(criteria[0], criteria[1], criteria[2])) {
        criteria = [criteria[0]];
    }
    return orderBy.orderBy(collection, flatten.flatten(criteria), ['asc']);
}

exports.sortBy = sortBy;
