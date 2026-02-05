import { differenceWith } from './differenceWith.mjs';
import { intersectionWith } from './intersectionWith.mjs';
import { unionWith } from './unionWith.mjs';

function xorWith(arr1, arr2, areElementsEqual) {
    const union = unionWith(arr1, arr2, areElementsEqual);
    const intersection = intersectionWith(arr1, arr2, areElementsEqual);
    return differenceWith(union, intersection, areElementsEqual);
}

export { xorWith };
