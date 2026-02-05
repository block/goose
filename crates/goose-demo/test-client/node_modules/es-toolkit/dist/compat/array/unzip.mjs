import { unzip as unzip$1 } from '../../array/unzip.mjs';
import { isArray } from '../predicate/isArray.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function unzip(array) {
    if (!isArrayLikeObject(array) || !array.length) {
        return [];
    }
    array = isArray(array) ? array : Array.from(array);
    array = array.filter(item => isArrayLikeObject(item));
    return unzip$1(array);
}

export { unzip };
