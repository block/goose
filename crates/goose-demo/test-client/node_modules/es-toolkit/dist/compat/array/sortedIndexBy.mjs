import { isNull } from '../../predicate/isNull.mjs';
import { isUndefined } from '../../predicate/isUndefined.mjs';
import { identity } from '../function/identity.mjs';
import { isNaN } from '../predicate/isNaN.mjs';
import { isNil } from '../predicate/isNil.mjs';
import { isSymbol } from '../predicate/isSymbol.mjs';
import { iteratee } from '../util/iteratee.mjs';

const MAX_ARRAY_LENGTH = 4294967295;
const MAX_ARRAY_INDEX = MAX_ARRAY_LENGTH - 1;
function sortedIndexBy(array, value, iteratee$1 = identity, retHighest) {
    if (isNil(array) || array.length === 0) {
        return 0;
    }
    let low = 0;
    let high = array.length;
    const iterateeFunction = iteratee(iteratee$1);
    const transformedValue = iterateeFunction(value);
    const valIsNaN = isNaN(transformedValue);
    const valIsNull = isNull(transformedValue);
    const valIsSymbol = isSymbol(transformedValue);
    const valIsUndefined = isUndefined(transformedValue);
    while (low < high) {
        let setLow;
        const mid = Math.floor((low + high) / 2);
        const computed = iterateeFunction(array[mid]);
        const othIsDefined = !isUndefined(computed);
        const othIsNull = isNull(computed);
        const othIsReflexive = !isNaN(computed);
        const othIsSymbol = isSymbol(computed);
        if (valIsNaN) {
            setLow = retHighest || othIsReflexive;
        }
        else if (valIsUndefined) {
            setLow = othIsReflexive && (retHighest || othIsDefined);
        }
        else if (valIsNull) {
            setLow = othIsReflexive && othIsDefined && (retHighest || !othIsNull);
        }
        else if (valIsSymbol) {
            setLow = othIsReflexive && othIsDefined && !othIsNull && (retHighest || !othIsSymbol);
        }
        else if (othIsNull || othIsSymbol) {
            setLow = false;
        }
        else {
            setLow = retHighest ? computed <= transformedValue : computed < transformedValue;
        }
        if (setLow) {
            low = mid + 1;
        }
        else {
            high = mid;
        }
    }
    return Math.min(high, MAX_ARRAY_INDEX);
}

export { sortedIndexBy };
