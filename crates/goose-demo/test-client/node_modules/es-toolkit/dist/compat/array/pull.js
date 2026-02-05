'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const pull$1 = require('../../array/pull.js');

function pull(arr, ...valuesToRemove) {
    return pull$1.pull(arr, valuesToRemove);
}

exports.pull = pull;
