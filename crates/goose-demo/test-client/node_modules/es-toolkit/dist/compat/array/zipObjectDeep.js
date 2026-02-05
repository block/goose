'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const zip = require('../../array/zip.js');
const set = require('../object/set.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function zipObjectDeep(keys, values) {
    const result = {};
    if (!isArrayLike.isArrayLike(keys)) {
        return result;
    }
    if (!isArrayLike.isArrayLike(values)) {
        values = [];
    }
    const zipped = zip.zip(Array.from(keys), Array.from(values));
    for (let i = 0; i < zipped.length; i++) {
        const [key, value] = zipped[i];
        if (key != null) {
            set.set(result, key, value);
        }
    }
    return result;
}

exports.zipObjectDeep = zipObjectDeep;
