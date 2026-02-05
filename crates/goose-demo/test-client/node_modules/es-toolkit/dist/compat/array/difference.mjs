import { difference as difference$1 } from '../../array/difference.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function difference(arr, ...values) {
    if (!isArrayLikeObject(arr)) {
        return [];
    }
    const arr1 = toArray(arr);
    const arr2 = [];
    for (let i = 0; i < values.length; i++) {
        const value = values[i];
        if (isArrayLikeObject(value)) {
            arr2.push(...Array.from(value));
        }
    }
    return difference$1(arr1, arr2);
}

export { difference };
