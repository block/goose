import { isDeepKey } from '../_internal/isDeepKey.mjs';
import { isIndex } from '../_internal/isIndex.mjs';
import { isArguments } from '../predicate/isArguments.mjs';
import { toPath } from '../util/toPath.mjs';

function hasIn(object, path) {
    if (object == null) {
        return false;
    }
    let resolvedPath;
    if (Array.isArray(path)) {
        resolvedPath = path;
    }
    else if (typeof path === 'string' && isDeepKey(path) && object[path] == null) {
        resolvedPath = toPath(path);
    }
    else {
        resolvedPath = [path];
    }
    if (resolvedPath.length === 0) {
        return false;
    }
    let current = object;
    for (let i = 0; i < resolvedPath.length; i++) {
        const key = resolvedPath[i];
        if (current == null || !(key in Object(current))) {
            const isSparseIndex = (Array.isArray(current) || isArguments(current)) && isIndex(key) && key < current.length;
            if (!isSparseIndex) {
                return false;
            }
        }
        current = current[key];
    }
    return true;
}

export { hasIn };
