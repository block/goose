'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const getTag = require('../_internal/getTag.js');

function isArguments(value) {
    return value !== null && typeof value === 'object' && getTag.getTag(value) === '[object Arguments]';
}

exports.isArguments = isArguments;
