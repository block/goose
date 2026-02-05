'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const intersectionBy$1 = require('../../array/intersectionBy.js');
const last = require('../../array/last.js');
const uniq = require('../../array/uniq.js');
const identity = require('../../function/identity.js');
const property = require('../object/property.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function intersectionBy(array, ...values) {
    if (!isArrayLikeObject.isArrayLikeObject(array)) {
        return [];
    }
    const lastValue = last.last(values);
    if (lastValue === undefined) {
        return Array.from(array);
    }
    let result = uniq.uniq(Array.from(array));
    const count = isArrayLikeObject.isArrayLikeObject(lastValue) ? values.length : values.length - 1;
    for (let i = 0; i < count; ++i) {
        const value = values[i];
        if (!isArrayLikeObject.isArrayLikeObject(value)) {
            return [];
        }
        if (isArrayLikeObject.isArrayLikeObject(lastValue)) {
            result = intersectionBy$1.intersectionBy(result, Array.from(value), identity.identity);
        }
        else if (typeof lastValue === 'function') {
            result = intersectionBy$1.intersectionBy(result, Array.from(value), value => lastValue(value));
        }
        else if (typeof lastValue === 'string') {
            result = intersectionBy$1.intersectionBy(result, Array.from(value), property.property(lastValue));
        }
    }
    return result;
}

exports.intersectionBy = intersectionBy;
