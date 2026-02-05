'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const partial$1 = require('../../function/partial.js');

function partial(func, ...partialArgs) {
    return partial$1.partialImpl(func, partial.placeholder, ...partialArgs);
}
partial.placeholder = Symbol('compat.partial.placeholder');

exports.partial = partial;
