import { last } from '../../array/last.mjs';
import { uniq } from '../../array/uniq.mjs';
import { uniqWith } from '../../array/uniqWith.mjs';
import { flattenArrayLike } from '../_internal/flattenArrayLike.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function unionWith(...values) {
    const lastValue = last(values);
    const flattened = flattenArrayLike(values);
    if (isArrayLikeObject(lastValue) || lastValue == null) {
        return uniq(flattened);
    }
    return uniqWith(flattened, lastValue);
}

export { unionWith };
