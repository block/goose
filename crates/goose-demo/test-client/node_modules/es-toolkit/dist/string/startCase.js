'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const words = require('./words.js');

function startCase(str) {
    const words$1 = words.words(str.trim());
    let result = '';
    for (let i = 0; i < words$1.length; i++) {
        const word = words$1[i];
        if (result) {
            result += ' ';
        }
        result += word[0].toUpperCase() + word.slice(1).toLowerCase();
    }
    return result;
}

exports.startCase = startCase;
