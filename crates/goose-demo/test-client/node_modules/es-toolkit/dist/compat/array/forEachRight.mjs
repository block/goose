import { identity } from '../../function/identity.mjs';
import { range } from '../../math/range.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function forEachRight(collection, callback = identity) {
    if (!collection) {
        return collection;
    }
    const keys = isArrayLike(collection) ? range(0, collection.length) : Object.keys(collection);
    for (let i = keys.length - 1; i >= 0; i--) {
        const key = keys[i];
        const value = collection[key];
        const result = callback(value, key, collection);
        if (result === false) {
            break;
        }
    }
    return collection;
}

export { forEachRight };
