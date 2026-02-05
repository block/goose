import { isArray } from '../compat/predicate/isArray.mjs';
import { isPlainObject } from '../predicate/isPlainObject.mjs';
import { camelCase } from '../string/camelCase.mjs';

function toCamelCaseKeys(obj) {
    if (isArray(obj)) {
        return obj.map(item => toCamelCaseKeys(item));
    }
    if (isPlainObject(obj)) {
        const result = {};
        const keys = Object.keys(obj);
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            const camelKey = camelCase(key);
            const convertedValue = toCamelCaseKeys(obj[key]);
            result[camelKey] = convertedValue;
        }
        return result;
    }
    return obj;
}

export { toCamelCaseKeys };
