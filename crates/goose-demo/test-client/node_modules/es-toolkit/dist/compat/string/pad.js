'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const pad$1 = require('../../string/pad.js');
const toString = require('../util/toString.js');

function pad(str, length, chars) {
    return pad$1.pad(toString.toString(str), length, chars);
}

exports.pad = pad;
