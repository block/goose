import { get } from './get.mjs';
import { isUnsafeProperty } from '../../_internal/isUnsafeProperty.mjs';
import { assignValue } from '../_internal/assignValue.mjs';
import { isIndex } from '../_internal/isIndex.mjs';
import { isKey } from '../_internal/isKey.mjs';
import { toKey } from '../_internal/toKey.mjs';
import { isObject } from '../predicate/isObject.mjs';
import { toPath } from '../util/toPath.mjs';

function updateWith(obj, path, updater, customizer) {
    if (obj == null && !isObject(obj)) {
        return obj;
    }
    let resolvedPath;
    if (isKey(path, obj)) {
        resolvedPath = [path];
    }
    else if (Array.isArray(path)) {
        resolvedPath = path;
    }
    else {
        resolvedPath = toPath(path);
    }
    const updateValue = updater(get(obj, resolvedPath));
    let current = obj;
    for (let i = 0; i < resolvedPath.length && current != null; i++) {
        const key = toKey(resolvedPath[i]);
        if (isUnsafeProperty(key)) {
            continue;
        }
        let newValue;
        if (i === resolvedPath.length - 1) {
            newValue = updateValue;
        }
        else {
            const objValue = current[key];
            const customizerResult = customizer?.(objValue, key, obj);
            newValue =
                customizerResult !== undefined
                    ? customizerResult
                    : isObject(objValue)
                        ? objValue
                        : isIndex(resolvedPath[i + 1])
                            ? []
                            : {};
        }
        assignValue(current, key, newValue);
        current = current[key];
    }
    return obj;
}

export { updateWith };
