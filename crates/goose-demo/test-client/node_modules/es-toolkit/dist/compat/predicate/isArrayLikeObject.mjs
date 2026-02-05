import { isArrayLike } from './isArrayLike.mjs';
import { isObjectLike } from './isObjectLike.mjs';

function isArrayLikeObject(value) {
    return isObjectLike(value) && isArrayLike(value);
}

export { isArrayLikeObject };
