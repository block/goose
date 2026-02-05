'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const deburr = require('./deburr.js');
const kebabCase$1 = require('../../string/kebabCase.js');
const normalizeForCase = require('../_internal/normalizeForCase.js');

function kebabCase(str) {
    return kebabCase$1.kebabCase(normalizeForCase.normalizeForCase(deburr.deburr(str)));
}

exports.kebabCase = kebabCase;
