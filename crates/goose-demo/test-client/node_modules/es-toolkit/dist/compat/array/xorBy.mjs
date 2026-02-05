import { differenceBy } from './differenceBy.mjs';
import { intersectionBy } from './intersectionBy.mjs';
import { last } from './last.mjs';
import { unionBy } from './unionBy.mjs';
import { windowed } from '../../array/windowed.mjs';
import { identity } from '../../function/identity.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function xorBy(...values) {
    const lastValue = last(values);
    let mapper = identity;
    if (!isArrayLikeObject(lastValue) && lastValue != null) {
        mapper = iteratee(lastValue);
        values = values.slice(0, -1);
    }
    const arrays = values.filter(isArrayLikeObject);
    const union = unionBy(...arrays, mapper);
    const intersections = windowed(arrays, 2).map(([arr1, arr2]) => intersectionBy(arr1, arr2, mapper));
    return differenceBy(union, unionBy(...intersections, mapper), mapper);
}

export { xorBy };
