'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isBuffer(x) {
    return typeof Buffer !== 'undefined' && Buffer.isBuffer(x);
}

exports.isBuffer = isBuffer;
