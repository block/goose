'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const isFunction = require('../../predicate/isFunction.js');
const forEach = require('../array/forEach.js');
const isBuffer = require('../predicate/isBuffer.js');
const isObject = require('../predicate/isObject.js');
const isTypedArray = require('../predicate/isTypedArray.js');
const iteratee = require('../util/iteratee.js');

function transform(object, iteratee$1 = identity.identity, accumulator) {
    const isArrayOrBufferOrTypedArray = Array.isArray(object) || isBuffer.isBuffer(object) || isTypedArray.isTypedArray(object);
    iteratee$1 = iteratee.iteratee(iteratee$1);
    if (accumulator == null) {
        if (isArrayOrBufferOrTypedArray) {
            accumulator = [];
        }
        else if (isObject.isObject(object) && isFunction.isFunction(object.constructor)) {
            accumulator = Object.create(Object.getPrototypeOf(object));
        }
        else {
            accumulator = {};
        }
    }
    if (object == null) {
        return accumulator;
    }
    forEach.forEach(object, (value, key, object) => iteratee$1(accumulator, value, key, object));
    return accumulator;
}

exports.transform = transform;
