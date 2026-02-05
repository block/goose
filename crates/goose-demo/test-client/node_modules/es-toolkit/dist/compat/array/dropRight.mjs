import { dropRight as dropRight$1 } from '../../array/dropRight.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { toInteger } from '../util/toInteger.mjs';

function dropRight(collection, itemsCount = 1, guard) {
    if (!isArrayLike(collection)) {
        return [];
    }
    itemsCount = guard ? 1 : toInteger(itemsCount);
    return dropRight$1(toArray(collection), itemsCount);
}

export { dropRight };
