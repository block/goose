import { identity } from '../../function/identity.mjs';
import { iteratee } from '../util/iteratee.mjs';

function find(source, _doesMatch = identity, fromIndex = 0) {
    if (!source) {
        return undefined;
    }
    if (fromIndex < 0) {
        fromIndex = Math.max(source.length + fromIndex, 0);
    }
    const doesMatch = iteratee(_doesMatch);
    if (!Array.isArray(source)) {
        const keys = Object.keys(source);
        for (let i = fromIndex; i < keys.length; i++) {
            const key = keys[i];
            const value = source[key];
            if (doesMatch(value, key, source)) {
                return value;
            }
        }
        return undefined;
    }
    return source.slice(fromIndex).find(doesMatch);
}

export { find };
