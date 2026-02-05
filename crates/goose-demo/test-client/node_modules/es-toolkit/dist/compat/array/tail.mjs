import { tail as tail$1 } from '../../array/tail.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function tail(arr) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return tail$1(toArray(arr));
}

export { tail };
