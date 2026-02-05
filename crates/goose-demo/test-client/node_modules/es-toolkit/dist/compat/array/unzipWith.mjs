import { unzip } from '../../array/unzip.mjs';
import { isArray } from '../predicate/isArray.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function unzipWith(array, iteratee) {
    if (!isArrayLikeObject(array) || !array.length) {
        return [];
    }
    const unzipped = isArray(array) ? unzip(array) : unzip(Array.from(array, value => Array.from(value)));
    if (!iteratee) {
        return unzipped;
    }
    const result = new Array(unzipped.length);
    for (let i = 0; i < unzipped.length; i++) {
        const value = unzipped[i];
        result[i] = iteratee(...value);
    }
    return result;
}

export { unzipWith };
