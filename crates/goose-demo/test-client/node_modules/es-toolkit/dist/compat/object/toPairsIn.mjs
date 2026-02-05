import { keysIn } from './keysIn.mjs';
import { mapToEntries } from '../_internal/mapToEntries.mjs';
import { setToEntries } from '../_internal/setToEntries.mjs';

function toPairsIn(object) {
    if (object == null) {
        return [];
    }
    if (object instanceof Set) {
        return setToEntries(object);
    }
    if (object instanceof Map) {
        return mapToEntries(object);
    }
    const keys = keysIn(object);
    const result = new Array(keys.length);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = object[key];
        result[i] = [key, value];
    }
    return result;
}

export { toPairsIn };
