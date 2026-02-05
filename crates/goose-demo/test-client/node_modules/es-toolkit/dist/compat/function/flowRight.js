'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('../../array/flatten.js');
const flowRight$1 = require('../../function/flowRight.js');

function flowRight(...funcs) {
    const flattenFuncs = flatten.flatten(funcs, 1);
    if (flattenFuncs.some(func => typeof func !== 'function')) {
        throw new TypeError('Expected a function');
    }
    return flowRight$1.flowRight(...flattenFuncs);
}

exports.flowRight = flowRight;
