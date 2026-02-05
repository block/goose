import { uniqWith } from './uniqWith.mjs';

function unionWith(arr1, arr2, areItemsEqual) {
    return uniqWith(arr1.concat(arr2), areItemsEqual);
}

export { unionWith };
