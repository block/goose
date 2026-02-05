'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const dropWhile$1 = require('../../array/dropWhile.js');
const identity = require('../../function/identity.js');
const toArray = require('../_internal/toArray.js');
const property = require('../object/property.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function dropWhile(arr, predicate = identity.identity) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return dropWhileImpl(toArray.toArray(arr), predicate);
}
function dropWhileImpl(arr, predicate) {
    switch (typeof predicate) {
        case 'function': {
            return dropWhile$1.dropWhile(arr, (item, index, arr) => Boolean(predicate(item, index, arr)));
        }
        case 'object': {
            if (Array.isArray(predicate) && predicate.length === 2) {
                const key = predicate[0];
                const value = predicate[1];
                return dropWhile$1.dropWhile(arr, matchesProperty.matchesProperty(key, value));
            }
            else {
                return dropWhile$1.dropWhile(arr, matches.matches(predicate));
            }
        }
        case 'number':
        case 'symbol':
        case 'string': {
            return dropWhile$1.dropWhile(arr, property.property(predicate));
        }
    }
}

exports.dropWhile = dropWhile;
