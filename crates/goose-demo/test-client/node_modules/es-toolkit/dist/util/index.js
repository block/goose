'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const attempt = require('./attempt.js');
const attemptAsync = require('./attemptAsync.js');
const invariant = require('./invariant.js');



exports.attempt = attempt.attempt;
exports.attemptAsync = attemptAsync.attemptAsync;
exports.assert = invariant.invariant;
exports.invariant = invariant.invariant;
