import { identity } from '../function/identity.mjs';
import { isObject } from '../predicate/isObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function findLastKey(obj, predicate) {
    if (!isObject(obj)) {
        return undefined;
    }
    const iteratee$1 = iteratee(predicate ?? identity);
    const keys = Object.keys(obj);
    return keys.findLast(key => iteratee$1(obj[key], key, obj));
}

export { findLastKey };
