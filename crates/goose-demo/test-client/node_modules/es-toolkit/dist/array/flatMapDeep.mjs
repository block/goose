import { flattenDeep } from './flattenDeep.mjs';

function flatMapDeep(arr, iteratee) {
    return flattenDeep(arr.map((item, index) => iteratee(item, index, arr)));
}

export { flatMapDeep };
