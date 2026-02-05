'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isSymbol = require('../predicate/isSymbol.js');

const regexIsDeepProp = /\.|\[(?:[^[\]]*|(["'])(?:(?!\1)[^\\]|\\.)*?\1)\]/;
const regexIsPlainProp = /^\w*$/;
function isKey(value, object) {
    if (Array.isArray(value)) {
        return false;
    }
    if (typeof value === 'number' || typeof value === 'boolean' || value == null || isSymbol.isSymbol(value)) {
        return true;
    }
    return ((typeof value === 'string' && (regexIsPlainProp.test(value) || !regexIsDeepProp.test(value))) ||
        (object != null && Object.hasOwn(object, value)));
}

exports.isKey = isKey;
