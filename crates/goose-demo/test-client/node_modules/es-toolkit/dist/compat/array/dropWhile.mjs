import { dropWhile as dropWhile$1 } from '../../array/dropWhile.mjs';
import { identity } from '../../function/identity.mjs';
import { toArray } from '../_internal/toArray.mjs';
import { property } from '../object/property.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

function dropWhile(arr, predicate = identity) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return dropWhileImpl(toArray(arr), predicate);
}
function dropWhileImpl(arr, predicate) {
    switch (typeof predicate) {
        case 'function': {
            return dropWhile$1(arr, (item, index, arr) => Boolean(predicate(item, index, arr)));
        }
        case 'object': {
            if (Array.isArray(predicate) && predicate.length === 2) {
                const key = predicate[0];
                const value = predicate[1];
                return dropWhile$1(arr, matchesProperty(key, value));
            }
            else {
                return dropWhile$1(arr, matches(predicate));
            }
        }
        case 'number':
        case 'symbol':
        case 'string': {
            return dropWhile$1(arr, property(predicate));
        }
    }
}

export { dropWhile };
