import { identity } from '../../function/identity.mjs';
import { range } from '../../math/range.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function reduceRight(collection, iteratee = identity, accumulator) {
    if (!collection) {
        return accumulator;
    }
    let keys;
    let startIndex;
    if (isArrayLike(collection)) {
        keys = range(0, collection.length).reverse();
        if (accumulator == null && collection.length > 0) {
            accumulator = collection[collection.length - 1];
            startIndex = 1;
        }
        else {
            startIndex = 0;
        }
    }
    else {
        keys = Object.keys(collection).reverse();
        if (accumulator == null) {
            accumulator = collection[keys[0]];
            startIndex = 1;
        }
        else {
            startIndex = 0;
        }
    }
    for (let i = startIndex; i < keys.length; i++) {
        const key = keys[i];
        const value = collection[key];
        accumulator = iteratee(accumulator, value, key, collection);
    }
    return accumulator;
}

export { reduceRight };
