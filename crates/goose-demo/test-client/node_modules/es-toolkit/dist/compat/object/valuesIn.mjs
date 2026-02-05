import { keysIn } from './keysIn.mjs';

function valuesIn(object) {
    const keys = keysIn(object);
    const result = new Array(keys.length);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        result[i] = object[key];
    }
    return result;
}

export { valuesIn };
