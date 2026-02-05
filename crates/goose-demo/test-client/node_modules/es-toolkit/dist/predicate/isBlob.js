'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isBlob(x) {
    if (typeof Blob === 'undefined') {
        return false;
    }
    return x instanceof Blob;
}

exports.isBlob = isBlob;
