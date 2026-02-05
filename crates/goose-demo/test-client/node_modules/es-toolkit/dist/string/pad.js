'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function pad(str, length, chars = ' ') {
    return str.padStart(Math.floor((length - str.length) / 2) + str.length, chars).padEnd(length, chars);
}

exports.pad = pad;
