'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const words = require('./words.js');

function constantCase(str) {
    const words$1 = words.words(str);
    return words$1.map(word => word.toUpperCase()).join('_');
}

exports.constantCase = constantCase;
