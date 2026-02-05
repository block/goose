import { isIndex } from './isIndex.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isObject } from '../predicate/isObject.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function isIterateeCall(value, index, object) {
    if (!isObject(object)) {
        return false;
    }
    if ((typeof index === 'number' && isArrayLike(object) && isIndex(index) && index < object.length) ||
        (typeof index === 'string' && index in object)) {
        return isEqualsSameValueZero(object[index], value);
    }
    return false;
}

export { isIterateeCall };
