'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

let idCounter = 0;
function uniqueId(prefix = '') {
    const id = ++idCounter;
    return `${prefix}${id}`;
}

exports.uniqueId = uniqueId;
