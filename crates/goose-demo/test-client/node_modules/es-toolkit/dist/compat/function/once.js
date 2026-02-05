'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const once$1 = require('../../function/once.js');

function once(func) {
    return once$1.once(func);
}

exports.once = once;
