import { identity } from '../../function/identity.mjs';
import { isIterateeCall } from '../_internal/isIterateeCall.mjs';
import { property } from '../object/property.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

function every(source, doesMatch, guard) {
    if (!source) {
        return true;
    }
    if (guard && isIterateeCall(source, doesMatch, guard)) {
        doesMatch = undefined;
    }
    if (!doesMatch) {
        doesMatch = identity;
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
                predicate = matchesProperty(key, value);
            }
            else {
                predicate = matches(doesMatch);
            }
            break;
        }
        case 'symbol':
        case 'number':
        case 'string': {
            predicate = property(doesMatch);
        }
    }
    if (!isArrayLike(source)) {
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

export { every };
