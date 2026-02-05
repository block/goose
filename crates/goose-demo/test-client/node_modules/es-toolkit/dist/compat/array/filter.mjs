import { identity } from '../../function/identity.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { iteratee } from '../util/iteratee.mjs';

function filter(source, predicate = identity) {
    if (!source) {
        return [];
    }
    predicate = iteratee(predicate);
    if (!Array.isArray(source)) {
        const result = [];
        const keys = Object.keys(source);
        const length = isArrayLike(source) ? source.length : keys.length;
        for (let i = 0; i < length; i++) {
            const key = keys[i];
            const value = source[key];
            if (predicate(value, key, source)) {
                result.push(value);
            }
        }
        return result;
    }
    const result = [];
    const length = source.length;
    for (let i = 0; i < length; i++) {
        const value = source[i];
        if (predicate(value, i, source)) {
            result.push(value);
        }
    }
    return result;
}

export { filter };
