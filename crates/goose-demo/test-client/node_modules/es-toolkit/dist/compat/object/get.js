'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isUnsafeProperty = require('../../_internal/isUnsafeProperty.js');
const isDeepKey = require('../_internal/isDeepKey.js');
const toKey = require('../_internal/toKey.js');
const toPath = require('../util/toPath.js');

function get(object, path, defaultValue) {
    if (object == null) {
        return defaultValue;
    }
    switch (typeof path) {
        case 'string': {
            if (isUnsafeProperty.isUnsafeProperty(path)) {
                return defaultValue;
            }
            const result = object[path];
            if (result === undefined) {
                if (isDeepKey.isDeepKey(path)) {
                    return get(object, toPath.toPath(path), defaultValue);
                }
                else {
                    return defaultValue;
                }
            }
            return result;
        }
        case 'number':
        case 'symbol': {
            if (typeof path === 'number') {
                path = toKey.toKey(path);
            }
            const result = object[path];
            if (result === undefined) {
                return defaultValue;
            }
            return result;
        }
        default: {
            if (Array.isArray(path)) {
                return getWithPath(object, path, defaultValue);
            }
            if (Object.is(path?.valueOf(), -0)) {
                path = '-0';
            }
            else {
                path = String(path);
            }
            if (isUnsafeProperty.isUnsafeProperty(path)) {
                return defaultValue;
            }
            const result = object[path];
            if (result === undefined) {
                return defaultValue;
            }
            return result;
        }
    }
}
function getWithPath(object, path, defaultValue) {
    if (path.length === 0) {
        return defaultValue;
    }
    let current = object;
    for (let index = 0; index < path.length; index++) {
        if (current == null) {
            return defaultValue;
        }
        if (isUnsafeProperty.isUnsafeProperty(path[index])) {
            return defaultValue;
        }
        current = current[path[index]];
    }
    if (current === undefined) {
        return defaultValue;
    }
    return current;
}

exports.get = get;
