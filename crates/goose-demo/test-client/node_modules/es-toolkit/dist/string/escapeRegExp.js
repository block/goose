'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function escapeRegExp(str) {
    return str.replace(/[\\^$.*+?()[\]{}|]/g, '\\$&');
}

exports.escapeRegExp = escapeRegExp;
