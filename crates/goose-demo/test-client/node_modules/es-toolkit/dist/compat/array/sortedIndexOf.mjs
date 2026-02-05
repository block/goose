import { sortedIndex } from './sortedIndex.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function sortedIndexOf(array, value) {
    if (!array?.length) {
        return -1;
    }
    const index = sortedIndex(array, value);
    if (index < array.length && isEqualsSameValueZero(array[index], value)) {
        return index;
    }
    return -1;
}

export { sortedIndexOf };
