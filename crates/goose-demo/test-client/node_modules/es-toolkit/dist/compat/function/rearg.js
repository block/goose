'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const flatten = require('../array/flatten.js');

function rearg(func, ...indices) {
    const flattenIndices = flatten.flatten(indices);
    return function (...args) {
        const reorderedArgs = flattenIndices.map(i => args[i]).slice(0, args.length);
        for (let i = reorderedArgs.length; i < args.length; i++) {
            reorderedArgs.push(args[i]);
        }
        return func.apply(this, reorderedArgs);
    };
}

exports.rearg = rearg;
