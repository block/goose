import { isObjectLike } from './isObjectLike.mjs';
import { isPlainObject } from './isPlainObject.mjs';

function isElement(value) {
    return isObjectLike(value) && value.nodeType === 1 && !isPlainObject(value);
}

export { isElement };
