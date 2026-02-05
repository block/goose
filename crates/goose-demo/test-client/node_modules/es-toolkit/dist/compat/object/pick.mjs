import { get } from './get.mjs';
import { has } from './has.mjs';
import { set } from './set.mjs';
import { isArrayLike } from '../predicate/isArrayLike.mjs';
import { isNil } from '../predicate/isNil.mjs';

function pick(obj, ...keysArr) {
    if (isNil(obj)) {
        return {};
    }
    const result = {};
    for (let i = 0; i < keysArr.length; i++) {
        let keys = keysArr[i];
        switch (typeof keys) {
            case 'object': {
                if (!Array.isArray(keys)) {
                    if (isArrayLike(keys)) {
                        keys = Array.from(keys);
                    }
                    else {
                        keys = [keys];
                    }
                }
                break;
            }
            case 'string':
            case 'symbol':
            case 'number': {
                keys = [keys];
                break;
            }
        }
        for (const key of keys) {
            const value = get(obj, key);
            if (value === undefined && !has(obj, key)) {
                continue;
            }
            if (typeof key === 'string' && Object.hasOwn(obj, key)) {
                result[key] = value;
            }
            else {
                set(result, key, value);
            }
        }
    }
    return result;
}

export { pick };
