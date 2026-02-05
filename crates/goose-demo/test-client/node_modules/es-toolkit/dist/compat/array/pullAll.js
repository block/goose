'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const pull = require('../../array/pull.js');

function pullAll(arr, valuesToRemove = []) {
    return pull.pull(arr, Array.from(valuesToRemove));
}

exports.pullAll = pullAll;
