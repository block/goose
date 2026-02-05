import { orderBy } from './orderBy.mjs';
import { flatten } from '../../array/flatten.mjs';
import { isIterateeCall } from '../_internal/isIterateeCall.mjs';

function sortBy(collection, ...criteria) {
    const length = criteria.length;
    if (length > 1 && isIterateeCall(collection, criteria[0], criteria[1])) {
        criteria = [];
    }
    else if (length > 2 && isIterateeCall(criteria[0], criteria[1], criteria[2])) {
        criteria = [criteria[0]];
    }
    return orderBy(collection, flatten(criteria), ['asc']);
}

export { sortBy };
