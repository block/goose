'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function constant(value) {
    return () => value;
}

exports.constant = constant;
