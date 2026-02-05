'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isPlainObject = require('../predicate/isPlainObject.js');

function defaultsDeep(target, ...sources) {
    target = Object(target);
    for (let i = 0; i < sources.length; i++) {
        const source = sources[i];
        if (source != null) {
            defaultsDeepRecursive(target, source, new WeakMap());
        }
    }
    return target;
}
function defaultsDeepRecursive(target, source, stack) {
    for (const key in source) {
        const sourceValue = source[key];
        const targetValue = target[key];
        if (targetValue === undefined || !Object.hasOwn(target, key)) {
            target[key] = handleMissingProperty(sourceValue, stack);
            continue;
        }
        if (stack.get(sourceValue) === targetValue) {
            continue;
        }
        handleExistingProperty(targetValue, sourceValue, stack);
    }
}
function handleMissingProperty(sourceValue, stack) {
    if (stack.has(sourceValue)) {
        return stack.get(sourceValue);
    }
    if (isPlainObject.isPlainObject(sourceValue)) {
        const newObj = {};
        stack.set(sourceValue, newObj);
        defaultsDeepRecursive(newObj, sourceValue, stack);
        return newObj;
    }
    return sourceValue;
}
function handleExistingProperty(targetValue, sourceValue, stack) {
    if (isPlainObject.isPlainObject(targetValue) && isPlainObject.isPlainObject(sourceValue)) {
        stack.set(sourceValue, targetValue);
        defaultsDeepRecursive(targetValue, sourceValue, stack);
        return;
    }
    if (Array.isArray(targetValue) && Array.isArray(sourceValue)) {
        stack.set(sourceValue, targetValue);
        mergeArrays(targetValue, sourceValue, stack);
    }
}
function mergeArrays(targetArray, sourceArray, stack) {
    const minLength = Math.min(sourceArray.length, targetArray.length);
    for (let i = 0; i < minLength; i++) {
        if (isPlainObject.isPlainObject(targetArray[i]) && isPlainObject.isPlainObject(sourceArray[i])) {
            defaultsDeepRecursive(targetArray[i], sourceArray[i], stack);
        }
    }
    for (let i = minLength; i < sourceArray.length; i++) {
        targetArray.push(sourceArray[i]);
    }
}

exports.defaultsDeep = defaultsDeep;
