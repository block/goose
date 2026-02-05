import { isFunction } from '../../predicate/isFunction.mjs';

function functionsIn(object) {
    if (object == null) {
        return [];
    }
    const result = [];
    for (const key in object) {
        if (isFunction(object[key])) {
            result.push(key);
        }
    }
    return result;
}

export { functionsIn };
