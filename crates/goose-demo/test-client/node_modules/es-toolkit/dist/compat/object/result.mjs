import { isKey } from '../_internal/isKey.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { toPath } from '../util/toPath.mjs';
import { toString } from '../util/toString.mjs';

function result(object, path, defaultValue) {
    if (isKey(path, object)) {
        path = [path];
    }
    else if (!Array.isArray(path)) {
        path = toPath(toString(path));
    }
    const pathLength = Math.max(path.length, 1);
    for (let index = 0; index < pathLength; index++) {
        const value = object == null ? undefined : object[toKey(path[index])];
        if (value === undefined) {
            return typeof defaultValue === 'function' ? defaultValue.call(object) : defaultValue;
        }
        object = typeof value === 'function' ? value.call(object) : value;
    }
    return object;
}

export { result };
