import { flatten } from './flatten.mjs';

function flattenDeep(arr) {
    return flatten(arr, Infinity);
}

export { flattenDeep };
