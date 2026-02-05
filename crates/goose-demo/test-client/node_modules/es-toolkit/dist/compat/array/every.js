'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const isIterateeCall = require('../_internal/isIterateeCall.js');
const property = require('../object/property.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function every(source, doesMatch, guard) {
    if (!source) {
        return true;
    }
    if (guard && isIterateeCall.isIterateeCall(source, doesMatch, guard)) {
        doesMatch = undefined;
    }
    if (!doesMatch) {
        doesMatch = identity.identity;
    }
    let predicate;
    switch (typeof doesMatch) {
        case 'function': {
            predicate = doesMatch;
            break;
        }
        case 'object': {
            if (Array.isArray(doesMatch) && doesMatch.length === 2) {
                const key = doesMatch[0];
                const value = doesMatch[1];
                predicate = matchesProperty.matchesProperty(key, value);
            }
            else {
                predicate = matches.matches(doesMatch);
            }
            break;
        }
        case 'symbol':
        case 'number':
        case 'string': {
            predicate = property.property(doesMatch);
        }
    }
    if (!isArrayLike.isArrayLike(source)) {
        const keys = Object.keys(source);
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            const value = source[key];
            if (!predicate(value, key, source)) {
                return false;
            }
        }
        return true;
    }
    for (let i = 0; i < source.length; i++) {
        if (!predicate(source[i], i, source)) {
            return false;
        }
    }
    return true;
}

exports.every = every;
