import { last } from '../../array/last.mjs';
import { uniq } from '../../array/uniq.mjs';
import { uniqBy } from '../../array/uniqBy.mjs';
import { ary } from '../../function/ary.mjs';
import { flattenArrayLike } from '../_internal/flattenArrayLike.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function unionBy(...values) {
    const lastValue = last(values);
    const flattened = flattenArrayLike(values);
    if (isArrayLikeObject(lastValue) || lastValue == null) {
        return uniq(flattened);
    }
    return uniqBy(flattened, ary(iteratee(lastValue), 1));
}

export { unionBy };
