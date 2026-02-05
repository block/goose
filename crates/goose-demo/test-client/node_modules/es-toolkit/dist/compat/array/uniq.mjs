import { uniq as uniq$1 } from '../../array/uniq.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function uniq(arr) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return uniq$1(Array.from(arr));
}

export { uniq };
