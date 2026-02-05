import { difference } from './difference.mjs';

function isSubset(superset, subset) {
    return difference(subset, superset).length === 0;
}

export { isSubset };
