import { differenceWith } from './differenceWith.mjs';
import { intersectionWith } from './intersectionWith.mjs';
import { last } from './last.mjs';
import { unionWith } from './unionWith.mjs';
import { windowed } from '../../array/windowed.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function xorWith(...values) {
    const lastValue = last(values);
    let comparator = (a, b) => a === b;
    if (typeof lastValue === 'function') {
        comparator = lastValue;
        values = values.slice(0, -1);
    }
    const arrays = values.filter(isArrayLikeObject);
    const union = unionWith(...arrays, comparator);
    const intersections = windowed(arrays, 2).map(([arr1, arr2]) => intersectionWith(arr1, arr2, comparator));
    return differenceWith(union, unionWith(...intersections, comparator), comparator);
}

export { xorWith };
