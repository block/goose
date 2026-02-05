'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flow = require('./flow.js');

function flowRight(...funcs) {
    return flow.flow(...funcs.reverse());
}

exports.flowRight = flowRight;
