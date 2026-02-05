import { isFunction } from '../../predicate/isFunction.mjs';
import { isNil } from '../../predicate/isNil.mjs';
import { get } from '../object/get.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';

function invokeMap(collection, path, ...args) {
    if (isNil(collection)) {
        return [];
    }
    const values = isArrayLike(collection) ? Array.from(collection) : Object.values(collection);
    const result = [];
    for (let i = 0; i < values.length; i++) {
        const value = values[i];
        if (isFunction(path)) {
            result.push(path.apply(value, args));
            continue;
        }
        const method = get(value, path);
        let thisContext = value;
        if (Array.isArray(path)) {
            const pathExceptLast = path.slice(0, -1);
            if (pathExceptLast.length > 0) {
                thisContext = get(value, pathExceptLast);
            }
        }
        else if (typeof path === 'string' && path.includes('.')) {
            const parts = path.split('.');
            const pathExceptLast = parts.slice(0, -1).join('.');
            thisContext = get(value, pathExceptLast);
        }
        result.push(method == null ? undefined : method.apply(thisContext, args));
    }
    return result;
}

export { invokeMap };
