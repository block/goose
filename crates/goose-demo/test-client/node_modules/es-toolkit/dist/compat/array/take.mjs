import { take as take$1 } from '../../array/take.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { toInteger } from '../util/toInteger.mjs';

function take(arr, count = 1, guard) {
    count = guard ? 1 : toInteger(count);
    if (count < 1 || !isArrayLike(arr)) {
        return [];
    }
    return take$1(toArray(arr), count);
}

export { take };
