'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function findKey(obj, predicate) {
    const keys = Object.keys(obj);
    return keys.find(key => predicate(obj[key], key, obj));
}

exports.findKey = findKey;
