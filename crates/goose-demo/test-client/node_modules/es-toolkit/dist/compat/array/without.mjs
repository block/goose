import { without as without$1 } from '../../array/without.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function without(array, ...values) {
    if (!isArrayLikeObject(array)) {
        return [];
    }
    return without$1(Array.from(array), ...values);
}

export { without };
