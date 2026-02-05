import { last } from './last.mjs';
import { difference } from '../../array/difference.mjs';
import { differenceBy as differenceBy$1 } from '../../array/differenceBy.mjs';
import { flattenArrayLike } from '../_internal/flattenArrayLike.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function differenceBy(arr, ..._values) {
    if (!isArrayLikeObject(arr)) {
        return [];
    }
    const iteratee$1 = last(_values);
    const values = flattenArrayLike(_values);
    if (isArrayLikeObject(iteratee$1)) {
        return difference(Array.from(arr), values);
    }
    return differenceBy$1(Array.from(arr), values, iteratee(iteratee$1));
}

export { differenceBy };
