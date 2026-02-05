import { identity } from '../../function/identity.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { iteratee } from '../util/iteratee.mjs';

function partition(source, predicate = identity) {
    if (!source) {
        return [[], []];
    }
    const collection = isArrayLike(source) ? source : Object.values(source);
    predicate = iteratee(predicate);
    const matched = [];
    const unmatched = [];
    for (let i = 0; i < collection.length; i++) {
        const value = collection[i];
        if (predicate(value)) {
            matched.push(value);
        }
        else {
            unmatched.push(value);
        }
    }
    return [matched, unmatched];
}

export { partition };
