'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const compat = require('./compat.js');

const toolkit = ((value) => {
    return value;
});
Object.assign(toolkit, compat);
toolkit.partial.placeholder = toolkit;
toolkit.partialRight.placeholder = toolkit;

exports.toolkit = toolkit;
