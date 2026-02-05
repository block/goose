import { findKey as findKey$1 } from '../../object/findKey.mjs';
import { identity } from '../function/identity.mjs';
import { isObject } from '../predicate/isObject.mjs';
import { iteratee } from '../util/iteratee.mjs';

function findKey(obj, predicate) {
    if (!isObject(obj)) {
        return undefined;
    }
    const iteratee$1 = iteratee(predicate ?? identity);
    return findKey$1(obj, iteratee$1);
}

export { findKey };
