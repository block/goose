'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const words = require('./words.js');

function upperCase(str) {
    const words$1 = words.words(str);
    let result = '';
    for (let i = 0; i < words$1.length; i++) {
        result += words$1[i].toUpperCase();
        if (i < words$1.length - 1) {
            result += ' ';
        }
    }
    return result;
}

exports.upperCase = upperCase;
