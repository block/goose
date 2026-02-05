import { last } from './last.mjs';
import { difference } from '../../array/difference.mjs';
import { differenceWith as differenceWith$1 } from '../../array/differenceWith.mjs';
import { flattenArrayLike } from '../_internal/flattenArrayLike.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function differenceWith(array, ...values) {
    if (!isArrayLikeObject(array)) {
        return [];
    }
    const comparator = last(values);
    const flattenedValues = flattenArrayLike(values);
    if (typeof comparator === 'function') {
        return differenceWith$1(Array.from(array), flattenedValues, comparator);
    }
    return difference(Array.from(array), flattenedValues);
}

export { differenceWith };
