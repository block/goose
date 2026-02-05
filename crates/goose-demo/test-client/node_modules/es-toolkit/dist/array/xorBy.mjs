import { differenceBy } from './differenceBy.mjs';
import { intersectionBy } from './intersectionBy.mjs';
import { unionBy } from './unionBy.mjs';

function xorBy(arr1, arr2, mapper) {
    const union = unionBy(arr1, arr2, mapper);
    const intersection = intersectionBy(arr1, arr2, mapper);
    return differenceBy(union, intersection, mapper);
}

export { xorBy };
