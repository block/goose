'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const iteratee = require('../util/iteratee.js');

function overArgs(func, ..._transforms) {
    if (typeof func !== 'function') {
        throw new TypeError('Expected a function');
    }
    const transforms = _transforms.flat();
    return function (...args) {
        const length = Math.min(args.length, transforms.length);
        const transformedArgs = [...args];
        for (let i = 0; i < length; i++) {
            const transform = iteratee.iteratee(transforms[i] ?? identity.identity);
            transformedArgs[i] = transform.call(this, args[i]);
        }
        return func.apply(this, transformedArgs);
    };
}

exports.overArgs = overArgs;
