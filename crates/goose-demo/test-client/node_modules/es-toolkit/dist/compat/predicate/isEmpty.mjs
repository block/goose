import { isArguments } from './isArguments.mjs';
import { isArrayLike } from './isArrayLike.mjs';
import { isTypedArray } from './isTypedArray.mjs';
import { isPrototype } from '../_internal/isPrototype.mjs';

function isEmpty(value) {
    if (value == null) {
        return true;
    }
    if (isArrayLike(value)) {
        if (typeof value.splice !== 'function' &&
            typeof value !== 'string' &&
            (typeof Buffer === 'undefined' || !Buffer.isBuffer(value)) &&
            !isTypedArray(value) &&
            !isArguments(value)) {
            return false;
        }
        return value.length === 0;
    }
    if (typeof value === 'object') {
        if (value instanceof Map || value instanceof Set) {
            return value.size === 0;
        }
        const keys = Object.keys(value);
        if (isPrototype(value)) {
            return keys.filter(x => x !== 'constructor').length === 0;
        }
        return keys.length === 0;
    }
    return true;
}

export { isEmpty };
