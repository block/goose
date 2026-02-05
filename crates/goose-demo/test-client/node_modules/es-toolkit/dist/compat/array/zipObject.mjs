import { assignValue } from '../_internal/assignValue.mjs';

function zipObject(keys = [], values = []) {
    const result = {};
    for (let i = 0; i < keys.length; i++) {
        assignValue(result, keys[i], values[i]);
    }
    return result;
}

export { zipObject };
