'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keys = require('./keys.js');

function functions(object) {
    if (object == null) {
        return [];
    }
    return keys.keys(object).filter(key => typeof object[key] === 'function');
}

exports.functions = functions;
