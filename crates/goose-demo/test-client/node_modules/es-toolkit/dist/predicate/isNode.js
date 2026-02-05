'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isNode() {
    return typeof process !== 'undefined' && process?.versions?.node != null;
}

exports.isNode = isNode;
