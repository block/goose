import { keysIn } from './keysIn.mjs';
import { range } from '../../math/range.mjs';
import { getSymbolsIn } from '../_internal/getSymbolsIn.mjs';
import { identity } from '../function/identity.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isSymbol } from '../predicate/isSymbol.mjs';
import { iteratee } from '../util/iteratee.mjs';

function pickBy(obj, shouldPick) {
    if (obj == null) {
        return {};
    }
    const predicate = iteratee(shouldPick ?? identity);
    const result = {};
    const keys = isArrayLike(obj) ? range(0, obj.length) : [...keysIn(obj), ...getSymbolsIn(obj)];
    for (let i = 0; i < keys.length; i++) {
        const key = (isSymbol(keys[i]) ? keys[i] : keys[i].toString());
        const value = obj[key];
        if (predicate(value, key, obj)) {
            result[key] = value;
        }
    }
    return result;
}

export { pickBy };
