import { flattenDepth } from './flattenDepth.mjs';
import { map } from './map.mjs';
import { isNil } from '../../predicate/isNil.mjs';

function flatMap(collection, iteratee) {
    if (isNil(collection)) {
        return [];
    }
    const mapped = isNil(iteratee) ? map(collection) : map(collection, iteratee);
    return flattenDepth(mapped, 1);
}

export { flatMap };
