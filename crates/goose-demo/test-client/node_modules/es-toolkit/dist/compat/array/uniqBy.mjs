import { uniqBy as uniqBy$1 } from '../../array/uniqBy.mjs';
import { ary } from '../../function/ary.mjs';
import { identity } from '../../function/identity.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function uniqBy(array, iteratee$1 = identity) {
    if (!isArrayLikeObject(array)) {
        return [];
    }
    return uniqBy$1(Array.from(array), ary(iteratee(iteratee$1), 1));
}

export { uniqBy };
