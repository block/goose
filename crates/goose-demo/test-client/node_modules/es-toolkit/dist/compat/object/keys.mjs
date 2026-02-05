import { isBuffer } from '../../predicate/isBuffer.mjs';
import { isPrototype } from '../_internal/isPrototype.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isTypedArray } from '../predicate/isTypedArray.mjs';
import { times } from '../util/times.mjs';

function keys(object) {
    if (isArrayLike(object)) {
        return arrayLikeKeys(object);
    }
    const result = Object.keys(Object(object));
    if (!isPrototype(object)) {
        return result;
    }
    return result.filter(key => key !== 'constructor');
}
function arrayLikeKeys(object) {
    const indices = times(object.length, index => `${index}`);
    const filteredKeys = new Set(indices);
    if (isBuffer(object)) {
        filteredKeys.add('offset');
        filteredKeys.add('parent');
    }
    if (isTypedArray(object)) {
        filteredKeys.add('buffer');
        filteredKeys.add('byteLength');
        filteredKeys.add('byteOffset');
    }
    const inheritedKeys = Object.keys(object).filter(key => !filteredKeys.has(key));
    if (Array.isArray(object)) {
        return [...indices, ...inheritedKeys];
    }
    return [...indices.filter(index => Object.hasOwn(object, index)), ...inheritedKeys];
}

export { keys };
