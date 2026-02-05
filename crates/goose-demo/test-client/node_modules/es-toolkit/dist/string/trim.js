'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const trimEnd = require('./trimEnd.js');
const trimStart = require('./trimStart.js');

function trim(str, chars) {
    if (chars === undefined) {
        return str.trim();
    }
    return trimStart.trimStart(trimEnd.trimEnd(str, chars), chars);
}

exports.trim = trim;
