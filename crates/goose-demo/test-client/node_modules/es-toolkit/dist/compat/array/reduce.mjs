import { identity } from '../../function/identity.mjs';
import { range } from '../../math/range.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function reduce(collection, iteratee = identity, accumulator) {
    if (!collection) {
        return accumulator;
    }
    let keys;
    let startIndex = 0;
    if (isArrayLike(collection)) {
        keys = range(0, collection.length);
        if (accumulator == null && collection.length > 0) {
            accumulator = collection[0];
            startIndex += 1;
        }
    }
    else {
        keys = Object.keys(collection);
        if (accumulator == null) {
            accumulator = collection[keys[0]];
            startIndex += 1;
        }
    }
    for (let i = startIndex; i < keys.length; i++) {
        const key = keys[i];
        const value = collection[key];
        accumulator = iteratee(accumulator, value, key, collection);
    }
    return accumulator;
}

export { reduce };
