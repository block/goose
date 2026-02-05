'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const get = require('./get.js');

function propertyOf(object) {
    return function (path) {
        return get.get(object, path);
    };
}

exports.propertyOf = propertyOf;
