'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const capitalize$1 = require('../../string/capitalize.js');
const toString = require('../util/toString.js');

function capitalize(str) {
    return capitalize$1.capitalize(toString.toString(str));
}

exports.capitalize = capitalize;
