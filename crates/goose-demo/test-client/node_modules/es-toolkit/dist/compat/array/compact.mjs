import { compact as compact$1 } from '../../array/compact.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function compact(arr) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return compact$1(Array.from(arr));
}

export { compact };
