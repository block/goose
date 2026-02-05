'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const invoke = require('./invoke.js');

function method(path, ...args) {
    return function (object) {
        return invoke.invoke(object, path, args);
    };
}

exports.method = method;
