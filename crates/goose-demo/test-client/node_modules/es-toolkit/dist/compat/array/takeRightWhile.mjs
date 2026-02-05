import { identity } from '../../function/identity.mjs';
import { negate } from '../../function/negate.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function takeRightWhile(_array, predicate) {
    if (!isArrayLikeObject(_array)) {
        return [];
    }
    const array = toArray(_array);
    const index = array.findLastIndex(negate(iteratee(predicate ?? identity)));
    return array.slice(index + 1);
}

export { takeRightWhile };
