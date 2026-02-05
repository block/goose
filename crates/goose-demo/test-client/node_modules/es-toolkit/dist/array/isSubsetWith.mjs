import { differenceWith } from './differenceWith.mjs';

function isSubsetWith(superset, subset, areItemsEqual) {
    return differenceWith(subset, superset, areItemsEqual).length === 0;
}

export { isSubsetWith };
