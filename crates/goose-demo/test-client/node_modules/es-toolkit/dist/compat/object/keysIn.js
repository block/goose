'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isBuffer = require('../../predicate/isBuffer.js');
const isPrototype = require('../_internal/isPrototype.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isTypedArray = require('../predicate/isTypedArray.js');
const times = require('../util/times.js');

function keysIn(object) {
    if (object == null) {
        return [];
    }
    switch (typeof object) {
        case 'object':
        case 'function': {
            if (isArrayLike.isArrayLike(object)) {
                return arrayLikeKeysIn(object);
            }
            if (isPrototype.isPrototype(object)) {
                return prototypeKeysIn(object);
            }
            return keysInImpl(object);
        }
        default: {
            return keysInImpl(Object(object));
        }
    }
}
function keysInImpl(object) {
    const result = [];
    for (const key in object) {
        result.push(key);
    }
    return result;
}
function prototypeKeysIn(object) {
    const keys = keysInImpl(object);
    return keys.filter(key => key !== 'constructor');
}
function arrayLikeKeysIn(object) {
    const indices = times.times(object.length, index => `${index}`);
    const filteredKeys = new Set(indices);
    if (isBuffer.isBuffer(object)) {
        filteredKeys.add('offset');
        filteredKeys.add('parent');
    }
    if (isTypedArray.isTypedArray(object)) {
        filteredKeys.add('buffer');
        filteredKeys.add('byteLength');
        filteredKeys.add('byteOffset');
    }
    const inheritedKeys = keysInImpl(object).filter(key => !filteredKeys.has(key));
    if (Array.isArray(object)) {
        return [...indices, ...inheritedKeys];
    }
    return [...indices.filter(index => Object.hasOwn(object, index)), ...inheritedKeys];
}

exports.keysIn = keysIn;
