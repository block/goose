'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const sampleSize$1 = require('../../array/sampleSize.js');
const isIterateeCall = require('../_internal/isIterateeCall.js');
const clamp = require('../math/clamp.js');
const toArray = require('../util/toArray.js');
const toInteger = require('../util/toInteger.js');

function sampleSize(collection, size, guard) {
    const arrayCollection = toArray.toArray(collection);
    if (guard ? isIterateeCall.isIterateeCall(collection, size, guard) : size === undefined) {
        size = 1;
    }
    else {
        size = clamp.clamp(toInteger.toInteger(size), 0, arrayCollection.length);
    }
    return sampleSize$1.sampleSize(arrayCollection, size);
}

exports.sampleSize = sampleSize;
