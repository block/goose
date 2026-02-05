import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isMap } from '../predicate/isMap.mjs';

function toArray(value) {
    if (value == null) {
        return [];
    }
    if (isArrayLike(value) || isMap(value)) {
        return Array.from(value);
    }
    if (typeof value === 'object') {
        return Object.values(value);
    }
    return [];
}

export { toArray };
