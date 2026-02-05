'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const trimStart$1 = require('../../string/trimStart.js');

function trimStart(str, chars, guard) {
    if (str == null) {
        return '';
    }
    if (guard != null || chars == null) {
        return str.toString().trimStart();
    }
    return trimStart$1.trimStart(str, chars.toString().split(''));
}

exports.trimStart = trimStart;
