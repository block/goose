'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const clone = require('./clone.js');
const mergeWith = require('./mergeWith.js');
const isPlainObject = require('../predicate/isPlainObject.js');

function toMerged(target, source) {
    return mergeWith.mergeWith(clone.clone(target), source, function mergeRecursively(targetValue, sourceValue) {
        if (Array.isArray(sourceValue)) {
            if (Array.isArray(targetValue)) {
                return mergeWith.mergeWith(clone.clone(targetValue), sourceValue, mergeRecursively);
            }
            else {
                return mergeWith.mergeWith([], sourceValue, mergeRecursively);
            }
        }
        else if (isPlainObject.isPlainObject(sourceValue)) {
            if (isPlainObject.isPlainObject(targetValue)) {
                return mergeWith.mergeWith(clone.clone(targetValue), sourceValue, mergeRecursively);
            }
            else {
                return mergeWith.mergeWith({}, sourceValue, mergeRecursively);
            }
        }
    });
}

exports.toMerged = toMerged;
