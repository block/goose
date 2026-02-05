import { isArray } from '../compat/predicate/isArray.mjs';
import { isPlainObject } from '../compat/predicate/isPlainObject.mjs';
import { snakeCase } from '../string/snakeCase.mjs';

function toSnakeCaseKeys(obj) {
    if (isArray(obj)) {
        return obj.map(item => toSnakeCaseKeys(item));
    }
    if (isPlainObject(obj)) {
        const result = {};
        const keys = Object.keys(obj);
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            const snakeKey = snakeCase(key);
            const convertedValue = toSnakeCaseKeys(obj[key]);
            result[snakeKey] = convertedValue;
        }
        return result;
    }
    return obj;
}

export { toSnakeCaseKeys };
