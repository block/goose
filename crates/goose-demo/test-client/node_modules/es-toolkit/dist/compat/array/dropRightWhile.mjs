import { dropRightWhile as dropRightWhile$1 } from '../../array/dropRightWhile.mjs';
import { identity } from '../../function/identity.mjs';
import { property } from '../object/property.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { matches } from '../predicate/matches.mjs';
import { matchesProperty } from '../predicate/matchesProperty.mjs';

function dropRightWhile(arr, predicate = identity) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return dropRightWhileImpl(Array.from(arr), predicate);
}
function dropRightWhileImpl(arr, predicate) {
    switch (typeof predicate) {
        case 'function': {
            return dropRightWhile$1(arr, (item, index, arr) => Boolean(predicate(item, index, arr)));
        }
        case 'object': {
            if (Array.isArray(predicate) && predicate.length === 2) {
                const key = predicate[0];
                const value = predicate[1];
                return dropRightWhile$1(arr, matchesProperty(key, value));
            }
            else {
                return dropRightWhile$1(arr, matches(predicate));
            }
        }
        case 'symbol':
        case 'number':
        case 'string': {
            return dropRightWhile$1(arr, property(predicate));
        }
    }
}

export { dropRightWhile };
