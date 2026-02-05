import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { iteratee } from '../util/iteratee.mjs';

function countBy(collection, iteratee$1) {
    if (collection == null) {
        return {};
    }
    const array = isArrayLike(collection) ? Array.from(collection) : Object.values(collection);
    const mapper = iteratee(iteratee$1 ?? undefined);
    const result = Object.create(null);
    for (let i = 0; i < array.length; i++) {
        const item = array[i];
        const key = mapper(item);
        result[key] = (result[key] ?? 0) + 1;
    }
    return result;
}

export { countBy };
