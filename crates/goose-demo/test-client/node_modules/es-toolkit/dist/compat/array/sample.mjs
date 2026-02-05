import { sample as sample$1 } from '../../array/sample.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function sample(collection) {
    if (collection == null) {
        return undefined;
    }
    if (isArrayLike(collection)) {
        return sample$1(toArray(collection));
    }
    return sample$1(Object.values(collection));
}

export { sample };
