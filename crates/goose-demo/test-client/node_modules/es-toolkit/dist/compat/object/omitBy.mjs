import { keysIn } from './keysIn.mjs';
import { range } from '../../math/range.mjs';
import { getSymbolsIn } from '../_internal/getSymbolsIn.mjs';
import { identity } from '../function/identity.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isSymbol } from '../predicate/isSymbol.mjs';
import { iteratee } from '../util/iteratee.mjs';

function omitBy(object, shouldOmit) {
    if (object == null) {
        return {};
    }
    const result = {};
    const predicate = iteratee(shouldOmit ?? identity);
    const keys = isArrayLike(object)
        ? range(0, object.length)
        : [...keysIn(object), ...getSymbolsIn(object)];
    for (let i = 0; i < keys.length; i++) {
        const key = (isSymbol(keys[i]) ? keys[i] : keys[i].toString());
        const value = object[key];
        if (!predicate(value, key, object)) {
            result[key] = value;
        }
    }
    return result;
}

export { omitBy };
