import { uniqBy } from './uniqBy.mjs';

function unionBy(arr1, arr2, mapper) {
    return uniqBy(arr1.concat(arr2), mapper);
}

export { unionBy };
