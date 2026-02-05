'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const toArray = require('../_internal/toArray.js');
const property = require('../object/property.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function findLastIndex(arr, doesMatch = identity.identity, fromIndex = arr ? arr.length - 1 : 0) {
    if (!arr) {
        return -1;
    }
    if (fromIndex < 0) {
        fromIndex = Math.max(arr.length + fromIndex, 0);
    }
    else {
        fromIndex = Math.min(fromIndex, arr.length - 1);
    }
    const subArray = toArray.toArray(arr).slice(0, fromIndex + 1);
    switch (typeof doesMatch) {
        case 'function': {
            return subArray.findLastIndex(doesMatch);
        }
        case 'object': {
            if (Array.isArray(doesMatch) && doesMatch.length === 2) {
                const key = doesMatch[0];
                const value = doesMatch[1];
                return subArray.findLastIndex(matchesProperty.matchesProperty(key, value));
            }
            else {
                return subArray.findLastIndex(matches.matches(doesMatch));
            }
        }
        case 'number':
        case 'symbol':
        case 'string': {
            return subArray.findLastIndex(property.property(doesMatch));
        }
    }
}

exports.findLastIndex = findLastIndex;
