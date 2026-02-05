import { orderBy } from './orderBy.mjs';

function sortBy(arr, criteria) {
    return orderBy(arr, criteria, ['asc']);
}

export { sortBy };
