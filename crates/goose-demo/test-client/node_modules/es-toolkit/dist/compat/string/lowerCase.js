'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const deburr = require('./deburr.js');
const lowerCase$1 = require('../../string/lowerCase.js');
const normalizeForCase = require('../_internal/normalizeForCase.js');

function lowerCase(str) {
    return lowerCase$1.lowerCase(normalizeForCase.normalizeForCase(deburr.deburr(str)));
}

exports.lowerCase = lowerCase;
