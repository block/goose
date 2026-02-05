import { identity } from '../../function/identity.mjs';
import { isNil } from '../../predicate/isNil.mjs';
import { iteratee } from '../util/iteratee.mjs';

function invertBy(object, iteratee$1) {
    const result = {};
    if (isNil(object)) {
        return result;
    }
    if (iteratee$1 == null) {
        iteratee$1 = identity;
    }
    const keys = Object.keys(object);
    const getString = iteratee(iteratee$1);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        const valueStr = getString(value);
        if (Array.isArray(result[valueStr])) {
            result[valueStr].push(key);
        }
        else {
            result[valueStr] = [key];
        }
    }
    return result;
}

export { invertBy };
