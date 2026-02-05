import { identity } from '../../function/identity.mjs';
import { isFunction } from '../../predicate/isFunction.mjs';
import { forEach } from '../array/forEach.mjs';
import { isBuffer } from '../predicate/isBuffer.mjs';
import { isObject } from '../predicate/isObject.mjs';
import { isTypedArray } from '../predicate/isTypedArray.mjs';
import { iteratee } from '../util/iteratee.mjs';

function transform(object, iteratee$1 = identity, accumulator) {
    const isArrayOrBufferOrTypedArray = Array.isArray(object) || isBuffer(object) || isTypedArray(object);
    iteratee$1 = iteratee(iteratee$1);
    if (accumulator == null) {
        if (isArrayOrBufferOrTypedArray) {
            accumulator = [];
        }
        else if (isObject(object) && isFunction(object.constructor)) {
            accumulator = Object.create(Object.getPrototypeOf(object));
        }
        else {
            accumulator = {};
        }
    }
    if (object == null) {
        return accumulator;
    }
    forEach(object, (value, key, object) => iteratee$1(accumulator, value, key, object));
    return accumulator;
}

export { transform };
