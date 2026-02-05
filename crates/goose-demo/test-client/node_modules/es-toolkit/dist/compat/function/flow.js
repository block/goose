'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('../../array/flatten.js');
const flow$1 = require('../../function/flow.js');

function flow(...funcs) {
    const flattenFuncs = flatten.flatten(funcs, 1);
    if (flattenFuncs.some(func => typeof func !== 'function')) {
        throw new TypeError('Expected a function');
    }
    return flow$1.flow(...flattenFuncs);
}

exports.flow = flow;
