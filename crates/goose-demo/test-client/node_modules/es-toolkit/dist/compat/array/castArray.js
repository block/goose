'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function castArray(value) {
    if (arguments.length === 0) {
        return [];
    }
    return Array.isArray(value) ? value : [value];
}

exports.castArray = castArray;
