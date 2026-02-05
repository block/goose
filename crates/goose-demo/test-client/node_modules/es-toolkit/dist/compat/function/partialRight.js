'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const partialRight$1 = require('../../function/partialRight.js');

function partialRight(func, ...partialArgs) {
    return partialRight$1.partialRightImpl(func, partialRight.placeholder, ...partialArgs);
}
partialRight.placeholder = Symbol('compat.partialRight.placeholder');

exports.partialRight = partialRight;
