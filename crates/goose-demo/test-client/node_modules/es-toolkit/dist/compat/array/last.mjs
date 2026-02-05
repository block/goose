import { last as last$1 } from '../../array/last.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function last(array) {
    if (!isArrayLike(array)) {
        return undefined;
    }
    return last$1(toArray(array));
}

export { last };
