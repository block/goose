'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const remove$1 = require('../../array/remove.js');
const identity = require('../../function/identity.js');
const iteratee = require('../util/iteratee.js');

function remove(arr, shouldRemoveElement = identity.identity) {
    return remove$1.remove(arr, iteratee.iteratee(shouldRemoveElement));
}

exports.remove = remove;
