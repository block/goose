import { isArrayLikeObject } from '../predicate/isArrayLikeObject.mjs';

function flattenArrayLike(values) {
    const result = [];
    for (let i = 0; i < values.length; i++) {
        const arrayLike = values[i];
        if (!isArrayLikeObject(arrayLike)) {
            continue;
        }
        for (let j = 0; j < arrayLike.length; j++) {
            result.push(arrayLike[j]);
        }
    }
    return result;
}

export { flattenArrayLike };
