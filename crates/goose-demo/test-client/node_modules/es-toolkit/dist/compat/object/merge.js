'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const mergeWith = require('./mergeWith.js');
const noop = require('../../function/noop.js');

function merge(object, ...sources) {
    return mergeWith.mergeWith(object, ...sources, noop.noop);
}

exports.merge = merge;
