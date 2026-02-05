import { takeRight as takeRight$1 } from '../../array/takeRight.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { toInteger } from '../util/toInteger.mjs';

function takeRight(arr, count = 1, guard) {
    count = guard ? 1 : toInteger(count);
    if (count <= 0 || !isArrayLike(arr)) {
        return [];
    }
    return takeRight$1(toArray(arr), count);
}

export { takeRight };
