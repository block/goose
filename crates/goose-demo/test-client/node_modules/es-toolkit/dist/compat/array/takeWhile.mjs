import { toArray } from '../_internal/toArray.mjs';
import { identity } from '../function/identity.mjs';
import { negate } from '../function/negate.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function takeWhile(array, predicate) {
    if (!isArrayLikeObject(array)) {
        return [];
    }
    const _array = toArray(array);
    const index = _array.findIndex(negate(iteratee(predicate ?? identity)));
    return index === -1 ? _array : _array.slice(0, index);
}

export { takeWhile };
