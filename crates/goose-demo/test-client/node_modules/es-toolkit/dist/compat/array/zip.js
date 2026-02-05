'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const zip$1 = require('../../array/zip.js');
const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');

function zip(...arrays) {
    if (!arrays.length) {
        return [];
    }
    return zip$1.zip(...arrays.filter(group => isArrayLikeObject.isArrayLikeObject(group)));
}

exports.zip = zip;
