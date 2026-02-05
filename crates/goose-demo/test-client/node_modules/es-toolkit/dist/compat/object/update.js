'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const updateWith = require('./updateWith.js');

function update(obj, path, updater) {
    return updateWith.updateWith(obj, path, updater, () => undefined);
}

exports.update = update;
