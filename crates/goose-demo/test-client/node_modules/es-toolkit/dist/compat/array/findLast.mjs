import { identity } from '../../function/identity.mjs';
import { iteratee } from '../util/iteratee.mjs';
import { toInteger } from '../util/toInteger.mjs';

function findLast(source, _doesMatch = identity, fromIndex) {
    if (!source) {
        return undefined;
    }
    const length = Array.isArray(source) ? source.length : Object.keys(source).length;
    fromIndex = toInteger(fromIndex ?? length - 1);
    if (fromIndex < 0) {
        fromIndex = Math.max(length + fromIndex, 0);
    }
    else {
        fromIndex = Math.min(fromIndex, length - 1);
    }
    const doesMatch = iteratee(_doesMatch);
    if (!Array.isArray(source)) {
        const keys = Object.keys(source);
        for (let i = fromIndex; i >= 0; i--) {
            const key = keys[i];
            const value = source[key];
            if (doesMatch(value, key, source)) {
                return value;
            }
        }
        return undefined;
    }
    return source.slice(0, fromIndex + 1).findLast(doesMatch);
}

export { findLast };
