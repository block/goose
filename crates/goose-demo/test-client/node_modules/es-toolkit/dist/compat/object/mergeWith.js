'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const cloneDeep = require('./cloneDeep.js');
const isUnsafeProperty = require('../../_internal/isUnsafeProperty.js');
const clone = require('../../object/clone.js');
const isPrimitive = require('../../predicate/isPrimitive.js');
const getSymbols = require('../_internal/getSymbols.js');
const isArguments = require('../predicate/isArguments.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const isObjectLike = require('../predicate/isObjectLike.js');
const isPlainObject = require('../predicate/isPlainObject.js');
const isTypedArray = require('../predicate/isTypedArray.js');

function mergeWith(object, ...otherArgs) {
    const sources = otherArgs.slice(0, -1);
    const merge = otherArgs[otherArgs.length - 1];
    let result = object;
    for (let i = 0; i < sources.length; i++) {
        const source = sources[i];
        result = mergeWithDeep(result, source, merge, new Map());
    }
    return result;
}
function mergeWithDeep(target, source, merge, stack) {
    if (isPrimitive.isPrimitive(target)) {
        target = Object(target);
    }
    if (source == null || typeof source !== 'object') {
        return target;
    }
    if (stack.has(source)) {
        return clone.clone(stack.get(source));
    }
    stack.set(source, target);
    if (Array.isArray(source)) {
        source = source.slice();
        for (let i = 0; i < source.length; i++) {
            source[i] = source[i] ?? undefined;
        }
    }
    const sourceKeys = [...Object.keys(source), ...getSymbols.getSymbols(source)];
    for (let i = 0; i < sourceKeys.length; i++) {
        const key = sourceKeys[i];
        if (isUnsafeProperty.isUnsafeProperty(key)) {
            continue;
        }
        let sourceValue = source[key];
        let targetValue = target[key];
        if (isArguments.isArguments(sourceValue)) {
            sourceValue = { ...sourceValue };
        }
        if (isArguments.isArguments(targetValue)) {
            targetValue = { ...targetValue };
        }
        if (typeof Buffer !== 'undefined' && Buffer.isBuffer(sourceValue)) {
            sourceValue = cloneDeep.cloneDeep(sourceValue);
        }
        if (Array.isArray(sourceValue)) {
            if (Array.isArray(targetValue)) {
                const cloned = [];
                const targetKeys = Reflect.ownKeys(targetValue);
                for (let i = 0; i < targetKeys.length; i++) {
                    const targetKey = targetKeys[i];
                    cloned[targetKey] = targetValue[targetKey];
                }
                targetValue = cloned;
            }
            else if (isArrayLikeObject.isArrayLikeObject(targetValue)) {
                const cloned = [];
                for (let i = 0; i < targetValue.length; i++) {
                    cloned[i] = targetValue[i];
                }
                targetValue = cloned;
            }
            else {
                targetValue = [];
            }
        }
        const merged = merge(targetValue, sourceValue, key, target, source, stack);
        if (merged !== undefined) {
            target[key] = merged;
        }
        else if (Array.isArray(sourceValue)) {
            target[key] = mergeWithDeep(targetValue, sourceValue, merge, stack);
        }
        else if (isObjectLike.isObjectLike(targetValue) &&
            isObjectLike.isObjectLike(sourceValue) &&
            (isPlainObject.isPlainObject(targetValue) ||
                isPlainObject.isPlainObject(sourceValue) ||
                isTypedArray.isTypedArray(targetValue) ||
                isTypedArray.isTypedArray(sourceValue))) {
            target[key] = mergeWithDeep(targetValue, sourceValue, merge, stack);
        }
        else if (targetValue == null && isPlainObject.isPlainObject(sourceValue)) {
            target[key] = mergeWithDeep({}, sourceValue, merge, stack);
        }
        else if (targetValue == null && isTypedArray.isTypedArray(sourceValue)) {
            target[key] = cloneDeep.cloneDeep(sourceValue);
        }
        else if (targetValue === undefined || sourceValue !== undefined) {
            target[key] = sourceValue;
        }
    }
    return target;
}

exports.mergeWith = mergeWith;
