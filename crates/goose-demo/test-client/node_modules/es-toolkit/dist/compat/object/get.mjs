import { isUnsafeProperty } from '../../_internal/isUnsafeProperty.mjs';
import { isDeepKey } from '../_internal/isDeepKey.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { toPath } from '../util/toPath.mjs';

function get(object, path, defaultValue) {
    if (object == null) {
        return defaultValue;
    }
    switch (typeof path) {
        case 'string': {
            if (isUnsafeProperty(path)) {
                return defaultValue;
            }
            const result = object[path];
            if (result === undefined) {
                if (isDeepKey(path)) {
                    return get(object, toPath(path), defaultValue);
                }
                else {
                    return defaultValue;
                }
            }
            return result;
        }
        case 'number':
        case 'symbol': {
            if (typeof path === 'number') {
                path = toKey(path);
            }
            const result = object[path];
            if (result === undefined) {
                return defaultValue;
            }
            return result;
        }
        default: {
            if (Array.isArray(path)) {
                return getWithPath(object, path, defaultValue);
            }
            if (Object.is(path?.valueOf(), -0)) {
                path = '-0';
            }
            else {
                path = String(path);
            }
            if (isUnsafeProperty(path)) {
                return defaultValue;
            }
            const result = object[path];
            if (result === undefined) {
                return defaultValue;
            }
            return result;
        }
    }
}
function getWithPath(object, path, defaultValue) {
    if (path.length === 0) {
        return defaultValue;
    }
    let current = object;
    for (let index = 0; index < path.length; index++) {
        if (current == null) {
            return defaultValue;
        }
        if (isUnsafeProperty(path[index])) {
            return defaultValue;
        }
        current = current[path[index]];
    }
    if (current === undefined) {
        return defaultValue;
    }
    return current;
}

export { get };
