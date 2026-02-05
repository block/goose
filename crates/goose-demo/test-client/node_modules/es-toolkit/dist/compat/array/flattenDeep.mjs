import { flattenDepth } from './flattenDepth.mjs';

function flattenDeep(value) {
    return flattenDepth(value, Infinity);
}

export { flattenDeep };
