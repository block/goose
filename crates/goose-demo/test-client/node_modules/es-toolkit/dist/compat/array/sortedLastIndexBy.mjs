import { sortedIndexBy } from './sortedIndexBy.mjs';

function sortedLastIndexBy(array, value, iteratee) {
    return sortedIndexBy(array, value, iteratee, true);
}

export { sortedLastIndexBy };
