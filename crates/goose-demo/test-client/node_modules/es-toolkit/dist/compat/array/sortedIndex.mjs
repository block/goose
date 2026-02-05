import { sortedIndexBy } from './sortedIndexBy.mjs';
import { isNil } from '../../predicate/isNil.mjs';
import { isNull } from '../../predicate/isNull.mjs';
import { isSymbol } from '../../predicate/isSymbol.mjs';
import { isNumber } from '../predicate/isNumber.mjs';

const MAX_ARRAY_LENGTH = 4294967295;
const HALF_MAX_ARRAY_LENGTH = MAX_ARRAY_LENGTH >>> 1;
function sortedIndex(array, value) {
    if (isNil(array)) {
        return 0;
    }
    let low = 0;
    let high = array.length;
    if (isNumber(value) && value === value && high <= HALF_MAX_ARRAY_LENGTH) {
        while (low < high) {
            const mid = (low + high) >>> 1;
            const compute = array[mid];
            if (!isNull(compute) && !isSymbol(compute) && compute < value) {
                low = mid + 1;
            }
            else {
                high = mid;
            }
        }
        return high;
    }
    return sortedIndexBy(array, value, value => value);
}

export { sortedIndex };
