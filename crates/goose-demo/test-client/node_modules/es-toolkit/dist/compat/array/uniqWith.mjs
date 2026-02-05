import { uniqWith as uniqWith$1 } from '../../array/uniqWith.mjs';
import { uniq } from './uniq.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function uniqWith(arr, comparator) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return typeof comparator === 'function' ? uniqWith$1(Array.from(arr), comparator) : uniq(Array.from(arr));
}

export { uniqWith };
