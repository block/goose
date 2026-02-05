import { cloneDeepWith } from './cloneDeepWith.mjs';
import { keysIn } from './keysIn.mjs';
import { unset } from './unset.mjs';
import { getSymbolsIn } from '../_internal/getSymbolsIn.mjs';
import { isDeepKey } from '../_internal/isDeepKey.mjs';
import { flatten } from '../array/flatten.mjs';
import { isPlainObject } from '../predicate/isPlainObject.mjs';

function omit(obj, ...keysArr) {
    if (obj == null) {
        return {};
    }
    keysArr = flatten(keysArr);
    const result = cloneInOmit(obj, keysArr);
    for (let i = 0; i < keysArr.length; i++) {
        let keys = keysArr[i];
        switch (typeof keys) {
            case 'object': {
                if (!Array.isArray(keys)) {
                    keys = Array.from(keys);
                }
                for (let j = 0; j < keys.length; j++) {
                    const key = keys[j];
                    unset(result, key);
                }
                break;
            }
            case 'string':
            case 'symbol':
            case 'number': {
                unset(result, keys);
                break;
            }
        }
    }
    return result;
}
function cloneInOmit(obj, keys) {
    const hasDeepKey = keys.some(key => Array.isArray(key) || isDeepKey(key));
    if (hasDeepKey) {
        return deepCloneInOmit(obj);
    }
    return shallowCloneInOmit(obj);
}
function shallowCloneInOmit(obj) {
    const result = {};
    const keysToCopy = [...keysIn(obj), ...getSymbolsIn(obj)];
    for (let i = 0; i < keysToCopy.length; i++) {
        const key = keysToCopy[i];
        result[key] = obj[key];
    }
    return result;
}
function deepCloneInOmit(obj) {
    const result = {};
    const keysToCopy = [...keysIn(obj), ...getSymbolsIn(obj)];
    for (let i = 0; i < keysToCopy.length; i++) {
        const key = keysToCopy[i];
        result[key] = cloneDeepWith(obj[key], valueToClone => {
            if (isPlainObject(valueToClone)) {
                return undefined;
            }
            return valueToClone;
        });
    }
    return result;
}

export { omit };
