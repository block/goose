import { isBuffer } from '../../predicate/isBuffer.mjs';
import { isPrototype } from '../_internal/isPrototype.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isTypedArray } from '../predicate/isTypedArray.mjs';
import { times } from '../util/times.mjs';

function keysIn(object) {
    if (object == null) {
        return [];
    }
    switch (typeof object) {
        case 'object':
        case 'function': {
            if (isArrayLike(object)) {
                return arrayLikeKeysIn(object);
            }
            if (isPrototype(object)) {
                return prototypeKeysIn(object);
            }
            return keysInImpl(object);
        }
        default: {
            return keysInImpl(Object(object));
        }
    }
}
function keysInImpl(object) {
    const result = [];
    for (const key in object) {
        result.push(key);
    }
    return result;
}
function prototypeKeysIn(object) {
    const keys = keysInImpl(object);
    return keys.filter(key => key !== 'constructor');
}
function arrayLikeKeysIn(object) {
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
    const inheritedKeys = keysInImpl(object).filter(key => !filteredKeys.has(key));
    if (Array.isArray(object)) {
        return [...indices, ...inheritedKeys];
    }
    return [...indices.filter(index => Object.hasOwn(object, index)), ...inheritedKeys];
}

export { keysIn };
