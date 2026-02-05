'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const get = require('./get.js');
const isUnsafeProperty = require('../../_internal/isUnsafeProperty.js');
const assignValue = require('../_internal/assignValue.js');
const isIndex = require('../_internal/isIndex.js');
const isKey = require('../_internal/isKey.js');
const toKey = require('../_internal/toKey.js');
const isObject = require('../predicate/isObject.js');
const toPath = require('../util/toPath.js');

function updateWith(obj, path, updater, customizer) {
    if (obj == null && !isObject.isObject(obj)) {
        return obj;
    }
    let resolvedPath;
    if (isKey.isKey(path, obj)) {
        resolvedPath = [path];
    }
    else if (Array.isArray(path)) {
        resolvedPath = path;
    }
    else {
        resolvedPath = toPath.toPath(path);
    }
    const updateValue = updater(get.get(obj, resolvedPath));
    let current = obj;
    for (let i = 0; i < resolvedPath.length && current != null; i++) {
        const key = toKey.toKey(resolvedPath[i]);
        if (isUnsafeProperty.isUnsafeProperty(key)) {
            continue;
        }
        let newValue;
        if (i === resolvedPath.length - 1) {
            newValue = updateValue;
        }
        else {
            const objValue = current[key];
            const customizerResult = customizer?.(objValue, key, obj);
            newValue =
                customizerResult !== undefined
                    ? customizerResult
                    : isObject.isObject(objValue)
                        ? objValue
                        : isIndex.isIndex(resolvedPath[i + 1])
                            ? []
                            : {};
        }
        assignValue.assignValue(current, key, newValue);
        current = current[key];
    }
    return obj;
}

exports.updateWith = updateWith;
