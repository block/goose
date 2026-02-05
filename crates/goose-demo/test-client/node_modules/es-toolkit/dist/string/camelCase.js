'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const capitalize = require('./capitalize.js');
const words = require('./words.js');

function camelCase(str) {
    const words$1 = words.words(str);
    if (words$1.length === 0) {
        return '';
    }
    const [first, ...rest] = words$1;
    return `${first.toLowerCase()}${rest.map(word => capitalize.capitalize(word)).join('')}`;
}

exports.camelCase = camelCase;
