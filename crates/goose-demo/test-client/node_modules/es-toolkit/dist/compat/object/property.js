'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const get = require('./get.js');

function property(path) {
    return function (object) {
        return get.get(object, path);
    };
}

exports.property = property;
