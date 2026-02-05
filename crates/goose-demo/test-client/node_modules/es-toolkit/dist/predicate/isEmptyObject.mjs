import { isPlainObject } from './isPlainObject.mjs';

function isEmptyObject(value) {
    return isPlainObject(value) && Object.keys(value).length === 0;
}

export { isEmptyObject };
