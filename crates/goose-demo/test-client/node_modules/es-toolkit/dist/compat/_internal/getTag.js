'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function getTag(value) {
    if (value == null) {
        return value === undefined ? '[object Undefined]' : '[object Null]';
    }
    return Object.prototype.toString.call(value);
}

exports.getTag = getTag;
