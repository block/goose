import { keys } from './keys.mjs';
import { assignValue } from '../_internal/assignValue.mjs';
import { isObject } from '../predicate/isObject.mjs';

function create(prototype, properties) {
    const proto = isObject(prototype) ? Object.create(prototype) : {};
    if (properties != null) {
        const propsKeys = keys(properties);
        for (let i = 0; i < propsKeys.length; i++) {
            const key = propsKeys[i];
            const propsValue = properties[key];
            assignValue(proto, key, propsValue);
        }
    }
    return proto;
}

export { create };
