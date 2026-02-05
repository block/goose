'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const invoke = require('./invoke.js');

function methodOf(object, ...args) {
    return function (path) {
        return invoke.invoke(object, path, args);
    };
}

exports.methodOf = methodOf;
