import { uniq } from './uniq.mjs';

function union(arr1, arr2) {
    return uniq(arr1.concat(arr2));
}

export { union };
