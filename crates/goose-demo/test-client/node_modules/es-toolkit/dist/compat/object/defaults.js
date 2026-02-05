'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isNil = require('../../predicate/isNil.js');
const isIterateeCall = require('../_internal/isIterateeCall.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

function defaults(object, ...sources) {
    object = Object(object);
    const objectProto = Object.prototype;
    let length = sources.length;
    const guard = length > 2 ? sources[2] : undefined;
    if (guard && isIterateeCall.isIterateeCall(sources[0], sources[1], guard)) {
        length = 1;
    }
    for (let i = 0; i < length; i++) {
        if (isNil.isNil(sources[i])) {
            continue;
        }
        const source = sources[i];
        const keys = Object.keys(source);
        for (let j = 0; j < keys.length; j++) {
            const key = keys[j];
            const value = object[key];
            if (value === undefined ||
                (!Object.hasOwn(object, key) && isEqualsSameValueZero.isEqualsSameValueZero(value, objectProto[key]))) {
                object[key] = source[key];
            }
        }
    }
    return object;
}

exports.defaults = defaults;
