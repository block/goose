'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keys = require('./keys.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

function assign(object, ...sources) {
    for (let i = 0; i < sources.length; i++) {
        assignImpl(object, sources[i]);
    }
    return object;
}
function assignImpl(object, source) {
    const keys$1 = keys.keys(source);
    for (let i = 0; i < keys$1.length; i++) {
        const key = keys$1[i];
        if (!(key in object) || !isEqualsSameValueZero.isEqualsSameValueZero(object[key], source[key])) {
            object[key] = source[key];
        }
    }
}

exports.assign = assign;
