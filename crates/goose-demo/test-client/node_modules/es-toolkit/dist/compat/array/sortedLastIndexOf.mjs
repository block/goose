import { sortedLastIndex } from './sortedLastIndex.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function sortedLastIndexOf(array, value) {
    if (!array?.length) {
        return -1;
    }
    const index = sortedLastIndex(array, value) - 1;
    if (index >= 0 && isEqualsSameValueZero(array[index], value)) {
        return index;
    }
    return -1;
}

export { sortedLastIndexOf };
