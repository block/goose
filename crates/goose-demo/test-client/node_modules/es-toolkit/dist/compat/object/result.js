'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isKey = require('../_internal/isKey.js');
const toKey = require('../_internal/toKey.js');
const toPath = require('../util/toPath.js');
const toString = require('../util/toString.js');

function result(object, path, defaultValue) {
    if (isKey.isKey(path, object)) {
        path = [path];
    }
    else if (!Array.isArray(path)) {
        path = toPath.toPath(toString.toString(path));
    }
    const pathLength = Math.max(path.length, 1);
    for (let index = 0; index < pathLength; index++) {
        const value = object == null ? undefined : object[toKey.toKey(path[index])];
        if (value === undefined) {
            return typeof defaultValue === 'function' ? defaultValue.call(object) : defaultValue;
        }
        object = typeof value === 'function' ? value.call(object) : value;
    }
    return object;
}

exports.result = result;
