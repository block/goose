'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isBuffer = require('../../predicate/isBuffer.js');
const isPrototype = require('../_internal/isPrototype.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isTypedArray = require('../predicate/isTypedArray.js');
const times = require('../util/times.js');

function keys(object) {
    if (isArrayLike.isArrayLike(object)) {
        return arrayLikeKeys(object);
    }
    const result = Object.keys(Object(object));
    if (!isPrototype.isPrototype(object)) {
        return result;
    }
    return result.filter(key => key !== 'constructor');
}
function arrayLikeKeys(object) {
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
    const inheritedKeys = Object.keys(object).filter(key => !filteredKeys.has(key));
    if (Array.isArray(object)) {
        return [...indices, ...inheritedKeys];
    }
    return [...indices.filter(index => Object.hasOwn(object, index)), ...inheritedKeys];
}

exports.keys = keys;
