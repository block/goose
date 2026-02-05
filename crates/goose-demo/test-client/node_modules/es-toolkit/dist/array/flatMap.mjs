import { flatten } from './flatten.mjs';

function flatMap(arr, iteratee, depth = 1) {
    return flatten(arr.map((item, index) => iteratee(item, index, arr)), depth);
}

export { flatMap };
