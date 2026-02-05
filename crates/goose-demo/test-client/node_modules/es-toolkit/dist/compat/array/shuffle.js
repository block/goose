'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const shuffle$1 = require('../../array/shuffle.js');
const values = require('../object/values.js');
const isArray = require('../predicate/isArray.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isNil = require('../predicate/isNil.js');
const isObjectLike = require('../predicate/isObjectLike.js');

function shuffle(collection) {
    if (isNil.isNil(collection)) {
        return [];
    }
    if (isArray.isArray(collection)) {
        return shuffle$1.shuffle(collection);
    }
    if (isArrayLike.isArrayLike(collection)) {
        return shuffle$1.shuffle(Array.from(collection));
    }
    if (isObjectLike.isObjectLike(collection)) {
        return shuffle$1.shuffle(values.values(collection));
    }
    return [];
}

exports.shuffle = shuffle;
