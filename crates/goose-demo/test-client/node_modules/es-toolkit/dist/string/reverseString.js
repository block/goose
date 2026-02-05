'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function reverseString(value) {
    return [...value].reverse().join('');
}

exports.reverseString = reverseString;
