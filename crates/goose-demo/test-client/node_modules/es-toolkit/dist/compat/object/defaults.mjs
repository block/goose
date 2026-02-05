import { isNil } from '../../predicate/isNil.mjs';
import { isIterateeCall } from '../_internal/isIterateeCall.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function defaults(object, ...sources) {
    object = Object(object);
    const objectProto = Object.prototype;
    let length = sources.length;
    const guard = length > 2 ? sources[2] : undefined;
    if (guard && isIterateeCall(sources[0], sources[1], guard)) {
        length = 1;
    }
    for (let i = 0; i < length; i++) {
        if (isNil(sources[i])) {
            continue;
        }
        const source = sources[i];
        const keys = Object.keys(source);
        for (let j = 0; j < keys.length; j++) {
            const key = keys[j];
            const value = object[key];
            if (value === undefined ||
                (!Object.hasOwn(object, key) && isEqualsSameValueZero(value, objectProto[key]))) {
                object[key] = source[key];
            }
        }
    }
    return object;
}

export { defaults };
