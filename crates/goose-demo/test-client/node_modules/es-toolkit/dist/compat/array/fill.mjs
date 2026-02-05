import { fill as fill$1 } from '../../array/fill.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isString } from '../predicate/isString.mjs';

function fill(array, value, start = 0, end = array ? array.length : 0) {
    if (!isArrayLike(array)) {
        return [];
    }
    if (isString(array)) {
        return array;
    }
    start = Math.floor(start);
    end = Math.floor(end);
    if (!start) {
        start = 0;
    }
    if (!end) {
        end = 0;
    }
    return fill$1(array, value, start, end);
}

export { fill };
