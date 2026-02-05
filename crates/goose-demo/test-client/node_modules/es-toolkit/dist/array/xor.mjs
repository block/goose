import { difference } from './difference.mjs';
import { intersection } from './intersection.mjs';
import { union } from './union.mjs';

function xor(arr1, arr2) {
    return difference(union(arr1, arr2), intersection(arr1, arr2));
}

export { xor };
