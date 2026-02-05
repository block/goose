import { get } from './get.mjs';
import { isUnsafeProperty } from '../../_internal/isUnsafeProperty.mjs';
import { isDeepKey } from '../_internal/isDeepKey.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { toPath } from '../util/toPath.mjs';

function unset(obj, path) {
    if (obj == null) {
        return true;
    }
    switch (typeof path) {
        case 'symbol':
        case 'number':
        case 'object': {
            if (Array.isArray(path)) {
                return unsetWithPath(obj, path);
            }
            if (typeof path === 'number') {
                path = toKey(path);
            }
            else if (typeof path === 'object') {
                if (Object.is(path?.valueOf(), -0)) {
                    path = '-0';
                }
                else {
                    path = String(path);
                }
            }
            if (isUnsafeProperty(path)) {
                return false;
            }
            if (obj?.[path] === undefined) {
                return true;
            }
            try {
                delete obj[path];
                return true;
            }
            catch {
                return false;
            }
        }
        case 'string': {
            if (obj?.[path] === undefined && isDeepKey(path)) {
                return unsetWithPath(obj, toPath(path));
            }
            if (isUnsafeProperty(path)) {
                return false;
            }
            try {
                delete obj[path];
                return true;
            }
            catch {
                return false;
            }
        }
    }
}
function unsetWithPath(obj, path) {
    const parent = path.length === 1 ? obj : get(obj, path.slice(0, -1));
    const lastKey = path[path.length - 1];
    if (parent?.[lastKey] === undefined) {
        return true;
    }
    if (isUnsafeProperty(lastKey)) {
        return false;
    }
    try {
        delete parent[lastKey];
        return true;
    }
    catch {
        return false;
    }
}

export { unset };
