'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isUnsafeProperty = require('../_internal/isUnsafeProperty.js');
const isPlainObject = require('../predicate/isPlainObject.js');

function mergeWith(target, source, merge) {
    const sourceKeys = Object.keys(source);
    for (let i = 0; i < sourceKeys.length; i++) {
        const key = sourceKeys[i];
        if (isUnsafeProperty.isUnsafeProperty(key)) {
            continue;
        }
        const sourceValue = source[key];
        const targetValue = target[key];
        const merged = merge(targetValue, sourceValue, key, target, source);
        if (merged !== undefined) {
            target[key] = merged;
        }
        else if (Array.isArray(sourceValue)) {
            if (Array.isArray(targetValue)) {
                target[key] = mergeWith(targetValue, sourceValue, merge);
            }
            else {
                target[key] = mergeWith([], sourceValue, merge);
            }
        }
        else if (isPlainObject.isPlainObject(sourceValue)) {
            if (isPlainObject.isPlainObject(targetValue)) {
                target[key] = mergeWith(targetValue, sourceValue, merge);
            }
            else {
                target[key] = mergeWith({}, sourceValue, merge);
            }
        }
        else if (targetValue === undefined || sourceValue !== undefined) {
            target[key] = sourceValue;
        }
    }
    return target;
}

exports.mergeWith = mergeWith;
