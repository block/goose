'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const trimEnd$1 = require('../../string/trimEnd.js');

function trimEnd(str, chars, guard) {
    if (str == null) {
        return '';
    }
    if (guard != null || chars == null) {
        return str.toString().trimEnd();
    }
    return trimEnd$1.trimEnd(str, chars.toString().split(''));
}

exports.trimEnd = trimEnd;
