import { keys } from './keys.mjs';
import { mapToEntries } from '../_internal/mapToEntries.mjs';
import { setToEntries } from '../_internal/setToEntries.mjs';

function toPairs(object) {
    if (object == null) {
        return [];
    }
    if (object instanceof Set) {
        return setToEntries(object);
    }
    if (object instanceof Map) {
        return mapToEntries(object);
    }
    const keys$1 = keys(object);
    const result = new Array(keys$1.length);
    for (let i = 0; i < keys$1.length; i++) {
        const key = keys$1[i];
        const value = object[key];
        result[i] = [key, value];
    }
    return result;
}

export { toPairs };
