'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isUnsafeProperty = require('../_internal/isUnsafeProperty.js');
const isPlainObject = require('../predicate/isPlainObject.js');

function merge(target, source) {
    const sourceKeys = Object.keys(source);
    for (let i = 0; i < sourceKeys.length; i++) {
        const key = sourceKeys[i];
        if (isUnsafeProperty.isUnsafeProperty(key)) {
            continue;
        }
        const sourceValue = source[key];
        const targetValue = target[key];
        if (isMergeableValue(sourceValue) && isMergeableValue(targetValue)) {
            target[key] = merge(targetValue, sourceValue);
        }
        else if (Array.isArray(sourceValue)) {
            target[key] = merge([], sourceValue);
        }
        else if (isPlainObject.isPlainObject(sourceValue)) {
            target[key] = merge({}, sourceValue);
        }
        else if (targetValue === undefined || sourceValue !== undefined) {
            target[key] = sourceValue;
        }
    }
    return target;
}
function isMergeableValue(value) {
    return isPlainObject.isPlainObject(value) || Array.isArray(value);
}

exports.merge = merge;
