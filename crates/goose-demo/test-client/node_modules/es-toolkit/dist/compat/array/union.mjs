import { flatMapDepth } from './flatMapDepth.mjs';
import { uniq } from '../../array/uniq.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function union(...arrays) {
    const validArrays = arrays.filter(isArrayLikeObject);
    const flattened = flatMapDepth(validArrays, v => Array.from(v), 1);
    return uniq(flattened);
}

export { union };
