import { identity } from '../../function/identity.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { property } from '../object/property.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

function findLastIndex(arr, doesMatch = identity, fromIndex = arr ? arr.length - 1 : 0) {
    if (!arr) {
        return -1;
    }
    if (fromIndex < 0) {
        fromIndex = Math.max(arr.length + fromIndex, 0);
    }
    else {
        fromIndex = Math.min(fromIndex, arr.length - 1);
    }
    const subArray = toArray(arr).slice(0, fromIndex + 1);
    switch (typeof doesMatch) {
        case 'function': {
            return subArray.findLastIndex(doesMatch);
        }
        case 'object': {
            if (Array.isArray(doesMatch) && doesMatch.length === 2) {
                const key = doesMatch[0];
                const value = doesMatch[1];
                return subArray.findLastIndex(matchesProperty(key, value));
            }
            else {
                return subArray.findLastIndex(matches(doesMatch));
            }
        }
        case 'number':
        case 'symbol':
        case 'string': {
            return subArray.findLastIndex(property(doesMatch));
        }
    }
}

export { findLastIndex };
