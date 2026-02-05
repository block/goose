import { intersectionBy as intersectionBy$1 } from '../../array/intersectionBy.mjs';
import { last } from '../../array/last.mjs';
import { uniq } from '../../array/uniq.mjs';
import { identity } from '../../function/identity.mjs';
import { property } from '../object/property.mjs';
import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function intersectionBy(array, ...values) {
    if (!isArrayLikeObject(array)) {
        return [];
    }
    const lastValue = last(values);
    if (lastValue === undefined) {
        return Array.from(array);
    }
    let result = uniq(Array.from(array));
    const count = isArrayLikeObject(lastValue) ? values.length : values.length - 1;
    for (let i = 0; i < count; ++i) {
        const value = values[i];
        if (!isArrayLikeObject(value)) {
            return [];
        }
        if (isArrayLikeObject(lastValue)) {
            result = intersectionBy$1(result, Array.from(value), identity);
        }
        else if (typeof lastValue === 'function') {
            result = intersectionBy$1(result, Array.from(value), value => lastValue(value));
        }
        else if (typeof lastValue === 'string') {
            result = intersectionBy$1(result, Array.from(value), property(lastValue));
        }
    }
    return result;
}

export { intersectionBy };
