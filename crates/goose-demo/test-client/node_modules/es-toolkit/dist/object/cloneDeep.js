'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const cloneDeepWith = require('./cloneDeepWith.js');

function cloneDeep(obj) {
    return cloneDeepWith.cloneDeepWithImpl(obj, undefined, obj, new Map(), undefined);
}

exports.cloneDeep = cloneDeep;
