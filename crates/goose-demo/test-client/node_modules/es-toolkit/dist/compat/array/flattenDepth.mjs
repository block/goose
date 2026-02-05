import { flatten } from './flatten.mjs';

function flattenDepth(array, depth = 1) {
    return flatten(array, depth);
}

export { flattenDepth };
