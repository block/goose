'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const capitalize = require('./capitalize.js');
const words = require('./words.js');

function pascalCase(str) {
    const words$1 = words.words(str);
    return words$1.map(word => capitalize.capitalize(word)).join('');
}

exports.pascalCase = pascalCase;
