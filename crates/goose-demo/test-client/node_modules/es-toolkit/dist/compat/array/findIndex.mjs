import { property } from '../object/property.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

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
                index = subArray.findIndex(matchesProperty(key, value));
            }
            else {
                index = subArray.findIndex(matches(doesMatch));
            }
            break;
        }
        case 'number':
        case 'symbol':
        case 'string': {
            index = subArray.findIndex(property(doesMatch));
        }
    }
    return index === -1 ? -1 : index + fromIndex;
}

export { findIndex };
