'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const cloneDeep = require('./cloneDeep.js');
const defaults = require('./defaults.js');

function toDefaulted(object, ...sources) {
    const cloned = cloneDeep.cloneDeep(object);
    return defaults.defaults(cloned, ...sources);
}

exports.toDefaulted = toDefaulted;
