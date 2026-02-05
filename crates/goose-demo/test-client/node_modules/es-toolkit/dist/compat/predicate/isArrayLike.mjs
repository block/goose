import { isLength } from '../../predicate/isLength.mjs';

function isArrayLike(value) {
    return value != null && typeof value !== 'function' && isLength(value.length);
}

export { isArrayLike };
