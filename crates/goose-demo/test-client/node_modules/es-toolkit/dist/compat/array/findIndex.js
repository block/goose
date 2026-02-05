'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const property = require('../object/property.js');
const matches = require('../predicate/matches.js');
const matchesProperty = require('../predicate/matchesProperty.js');

function findIndex(arr, doesMatch, fromIndex = 0) {
    if (!arr) {
        return -1;
    }
    if (fromIndex < 0) {
        fromIndex = Math.max(arr.length + fromIndex, 0);
    }
    const subArray = Array.from(arr).slice(fromIndex);
    let index = -1;
    switch (typeof doesMatch) {
        case 'function': {
            index = subArray.findIndex(doesMatch);
            break;
        }
        case 'object': {
            if (Array.isArray(doesMatch) && doesMatch.length === 2) {
                const key = doesMatch[0];
                const value = doesMatch[1];
                index = subArray.findIndex(matchesProperty.matchesProperty(key, value));
            }
            else {
                index = subArray.findIndex(matches.matches(doesMatch));
            }
            break;
        }
        case 'number':
        case 'symbol':
        case 'string': {
            index = subArray.findIndex(property.property(doesMatch));
        }
    }
    return index === -1 ? -1 : index + fromIndex;
}

exports.findIndex = findIndex;
