'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const deburr = require('./deburr.js');
const upperCase$1 = require('../../string/upperCase.js');
const normalizeForCase = require('../_internal/normalizeForCase.js');

function upperCase(str) {
    return upperCase$1.upperCase(normalizeForCase.normalizeForCase(deburr.deburr(str)));
}

exports.upperCase = upperCase;
