import { chunk as chunk$1 } from '../../array/chunk.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function chunk(arr, size = 1) {
    size = Math.max(Math.floor(size), 0);
    if (size === 0 || !isArrayLike(arr)) {
        return [];
    }
    return chunk$1(toArray(arr), size);
}

export { chunk };
