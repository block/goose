import { intersection as intersection$1 } from '../../array/intersection.mjs';
import { uniq } from '../../array/uniq.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function intersection(...arrays) {
    if (arrays.length === 0) {
        return [];
    }
    if (!isArrayLikeObject(arrays[0])) {
        return [];
    }
    let result = uniq(Array.from(arrays[0]));
    for (let i = 1; i < arrays.length; i++) {
        const array = arrays[i];
        if (!isArrayLikeObject(array)) {
            return [];
        }
        result = intersection$1(result, Array.from(array));
    }
    return result;
}

export { intersection };
