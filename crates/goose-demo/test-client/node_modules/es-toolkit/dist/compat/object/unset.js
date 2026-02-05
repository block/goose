'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const get = require('./get.js');
const isUnsafeProperty = require('../../_internal/isUnsafeProperty.js');
const isDeepKey = require('../_internal/isDeepKey.js');
const toKey = require('../_internal/toKey.js');
const toPath = require('../util/toPath.js');

function unset(obj, path) {
    if (obj == null) {
        return true;
    }
    switch (typeof path) {
        case 'symbol':
        case 'number':
        case 'object': {
            if (Array.isArray(path)) {
                return unsetWithPath(obj, path);
            }
            if (typeof path === 'number') {
                path = toKey.toKey(path);
            }
            else if (typeof path === 'object') {
                if (Object.is(path?.valueOf(), -0)) {
                    path = '-0';
                }
                else {
                    path = String(path);
                }
            }
            if (isUnsafeProperty.isUnsafeProperty(path)) {
                return false;
            }
            if (obj?.[path] === undefined) {
                return true;
            }
            try {
                delete obj[path];
                return true;
            }
            catch {
                return false;
            }
        }
        case 'string': {
            if (obj?.[path] === undefined && isDeepKey.isDeepKey(path)) {
                return unsetWithPath(obj, toPath.toPath(path));
            }
            if (isUnsafeProperty.isUnsafeProperty(path)) {
                return false;
            }
            try {
                delete obj[path];
                return true;
            }
            catch {
                return false;
            }
        }
    }
}
function unsetWithPath(obj, path) {
    const parent = path.length === 1 ? obj : get.get(obj, path.slice(0, -1));
    const lastKey = path[path.length - 1];
    if (parent?.[lastKey] === undefined) {
        return true;
    }
    if (isUnsafeProperty.isUnsafeProperty(lastKey)) {
        return false;
    }
    try {
        delete parent[lastKey];
        return true;
    }
    catch {
        return false;
    }
}

exports.unset = unset;
