import { isDeepKey } from '../_internal/isDeepKey.mjs';
import { isIndex } from '../_internal/isIndex.mjs';
import { isArguments } from '../predicate/isArguments.mjs';
import { toPath } from '../util/toPath.mjs';

function has(object, path) {
    let resolvedPath;
    if (Array.isArray(path)) {
        resolvedPath = path;
    }
    else if (typeof path === 'string' && isDeepKey(path) && object?.[path] == null) {
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
        if (current == null || !Object.hasOwn(current, key)) {
            const isSparseIndex = (Array.isArray(current) || isArguments(current)) && isIndex(key) && key < current.length;
            if (!isSparseIndex) {
                return false;
            }
        }
        current = current[key];
    }
    return true;
}

export { has };
