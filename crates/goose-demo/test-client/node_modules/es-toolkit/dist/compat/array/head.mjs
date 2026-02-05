import { head as head$1 } from '../../array/head.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function head(arr) {
    if (!isArrayLike(arr)) {
        return undefined;
    }
    return head$1(toArray(arr));
}

export { head };
