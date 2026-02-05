import { isArrayLike } from '../predicate/isArrayLike.mjs';

function indexOf(array, searchElement, fromIndex) {
    if (!isArrayLike(array)) {
        return -1;
    }
    if (Number.isNaN(searchElement)) {
        fromIndex = fromIndex ?? 0;
        if (fromIndex < 0) {
            fromIndex = Math.max(0, array.length + fromIndex);
        }
        for (let i = fromIndex; i < array.length; i++) {
            if (Number.isNaN(array[i])) {
                return i;
            }
        }
        return -1;
    }
    return Array.from(array).indexOf(searchElement, fromIndex);
}

export { indexOf };
