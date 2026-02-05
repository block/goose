import { identity } from '../../function/identity.mjs';
import { range } from '../../math/range.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { iteratee } from '../util/iteratee.mjs';

function map(collection, _iteratee) {
    if (!collection) {
        return [];
    }
    const keys = isArrayLike(collection) || Array.isArray(collection) ? range(0, collection.length) : Object.keys(collection);
    const iteratee$1 = iteratee(_iteratee ?? identity);
    const result = new Array(keys.length);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = collection[key];
        result[i] = iteratee$1(value, key, collection);
    }
    return result;
}

export { map };
