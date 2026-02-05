'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const cloneDeepWith = require('./cloneDeepWith.js');

function cloneDeep(obj) {
    return cloneDeepWith.cloneDeepWith(obj);
}

exports.cloneDeep = cloneDeep;
