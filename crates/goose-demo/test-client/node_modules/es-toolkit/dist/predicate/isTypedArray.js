'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isTypedArray(x) {
    return ArrayBuffer.isView(x) && !(x instanceof DataView);
}

exports.isTypedArray = isTypedArray;
