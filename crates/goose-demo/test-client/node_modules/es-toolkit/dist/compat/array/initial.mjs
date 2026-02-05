import { initial as initial$1 } from '../../array/initial.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function initial(arr) {
    if (!isArrayLike(arr)) {
        return [];
    }
    return initial$1(Array.from(arr));
}

export { initial };
