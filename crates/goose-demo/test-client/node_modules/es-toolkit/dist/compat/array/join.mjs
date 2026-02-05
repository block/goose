import { isArrayLike } from '../predicate/isArrayLike.mjs';

function join(array, separator) {
    if (!isArrayLike(array)) {
        return '';
    }
    return Array.from(array).join(separator);
}

export { join };
