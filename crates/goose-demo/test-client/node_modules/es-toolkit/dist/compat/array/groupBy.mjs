import { groupBy as groupBy$1 } from '../../array/groupBy.mjs';
import { identity } from '../../function/identity.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { iteratee } from '../util/iteratee.mjs';

function groupBy(source, _getKeyFromItem) {
    if (source == null) {
        return {};
    }
    const items = isArrayLike(source) ? Array.from(source) : Object.values(source);
    const getKeyFromItem = iteratee(_getKeyFromItem ?? identity);
    return groupBy$1(items, getKeyFromItem);
}

export { groupBy };
