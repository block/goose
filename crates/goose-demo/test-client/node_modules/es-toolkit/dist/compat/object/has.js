'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isDeepKey = require('../_internal/isDeepKey.js');
const isIndex = require('../_internal/isIndex.js');
const isArguments = require('../predicate/isArguments.js');
const toPath = require('../util/toPath.js');

function has(object, path) {
    let resolvedPath;
    if (Array.isArray(path)) {
        resolvedPath = path;
    }
    else if (typeof path === 'string' && isDeepKey.isDeepKey(path) && object?.[path] == null) {
        resolvedPath = toPath.toPath(path);
    }
    else {
        resolvedPath = [path];
    }
    if (resolvedPath.length === 0) {
        return false;
    }
    let current = object;
    for (let i = 0; i < resolvedPath.length; i++) {
        const key = resolvedPath[i];
        if (current == null || !Object.hasOwn(current, key)) {
            const isSparseIndex = (Array.isArray(current) || isArguments.isArguments(current)) && isIndex.isIndex(key) && key < current.length;
            if (!isSparseIndex) {
                return false;
            }
        }
        current = current[key];
    }
    return true;
}

exports.has = has;
