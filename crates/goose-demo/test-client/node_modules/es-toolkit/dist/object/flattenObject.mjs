import { isPlainObject } from '../predicate/isPlainObject.mjs';

function flattenObject(object, { delimiter = '.' } = {}) {
    return flattenObjectImpl(object, '', delimiter);
}
function flattenObjectImpl(object, prefix, delimiter) {
    const result = {};
    const keys = Object.keys(object);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        const prefixedKey = prefix ? `${prefix}${delimiter}${key}` : key;
        if (isPlainObject(value) && Object.keys(value).length > 0) {
            Object.assign(result, flattenObjectImpl(value, prefixedKey, delimiter));
            continue;
        }
        if (Array.isArray(value) && value.length > 0) {
            Object.assign(result, flattenObjectImpl(value, prefixedKey, delimiter));
            continue;
        }
        result[prefixedKey] = value;
    }
    return result;
}

export { flattenObject };
