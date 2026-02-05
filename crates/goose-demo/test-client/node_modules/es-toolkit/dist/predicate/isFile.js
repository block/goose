'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isBlob = require('./isBlob.js');

function isFile(x) {
    if (typeof File === 'undefined') {
        return false;
    }
    return isBlob.isBlob(x) && x instanceof File;
}

exports.isFile = isFile;
