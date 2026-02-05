'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const dropRightWhile$1 = require('../../array/dropRightWhile.js');
const identity = require('../../function/identity.js');
const property = require('../object/property.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function dropRightWhile(arr, predicate = identity.identity) {
    if (!isArrayLike.isArrayLike(arr)) {
        return [];
    }
    return dropRightWhileImpl(Array.from(arr), predicate);
}
function dropRightWhileImpl(arr, predicate) {
    switch (typeof predicate) {
        case 'function': {
            return dropRightWhile$1.dropRightWhile(arr, (item, index, arr) => Boolean(predicate(item, index, arr)));
        }
        case 'object': {
            if (Array.isArray(predicate) && predicate.length === 2) {
                const key = predicate[0];
                const value = predicate[1];
                return dropRightWhile$1.dropRightWhile(arr, matchesProperty.matchesProperty(key, value));
            }
            else {
                return dropRightWhile$1.dropRightWhile(arr, matches.matches(predicate));
            }
        }
        case 'symbol':
        case 'number':
        case 'string': {
            return dropRightWhile$1.dropRightWhile(arr, property.property(predicate));
        }
    }
}

exports.dropRightWhile = dropRightWhile;
