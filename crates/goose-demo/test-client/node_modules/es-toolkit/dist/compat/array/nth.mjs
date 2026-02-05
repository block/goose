import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { toInteger } from '../util/toInteger.mjs';

function nth(array, n = 0) {
    if (!isArrayLikeObject(array) || array.length === 0) {
        return undefined;
    }
    n = toInteger(n);
    if (n < 0) {
        n += array.length;
    }
    return array[n];
}

export { nth };
