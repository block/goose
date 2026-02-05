'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const filter = require('./filter.js');
const identity = require('../../function/identity.js');
const negate = require('../function/negate.js');
const iteratee = require('../util/iteratee.js');

function reject(source, predicate = identity.identity) {
    return filter.filter(source, negate.negate(iteratee.iteratee(predicate)));
}

exports.reject = reject;
