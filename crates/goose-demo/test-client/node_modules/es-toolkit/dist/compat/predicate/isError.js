'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const getTag = require('../_internal/getTag.js');

function isError(value) {
    return getTag.getTag(value) === '[object Error]';
}

exports.isError = isError;
