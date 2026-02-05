'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const updateWith = require('./updateWith.js');

function set(obj, path, value) {
    return updateWith.updateWith(obj, path, () => value, () => undefined);
}

exports.set = set;
