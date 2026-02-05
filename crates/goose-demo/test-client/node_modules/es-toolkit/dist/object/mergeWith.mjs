import { isUnsafeProperty } from '../_internal/isUnsafeProperty.mjs';
import { isPlainObject } from '../predicate/isPlainObject.mjs';

function mergeWith(target, source, merge) {
    const sourceKeys = Object.keys(source);
    for (let i = 0; i < sourceKeys.length; i++) {
        const key = sourceKeys[i];
        if (isUnsafeProperty(key)) {
            continue;
        }
        const sourceValue = source[key];
        const targetValue = target[key];
        const merged = merge(targetValue, sourceValue, key, target, source);
        if (merged !== undefined) {
            target[key] = merged;
        }
        else if (Array.isArray(sourceValue)) {
            if (Array.isArray(targetValue)) {
                target[key] = mergeWith(targetValue, sourceValue, merge);
            }
            else {
                target[key] = mergeWith([], sourceValue, merge);
            }
        }
        else if (isPlainObject(sourceValue)) {
            if (isPlainObject(targetValue)) {
                target[key] = mergeWith(targetValue, sourceValue, merge);
            }
            else {
                target[key] = mergeWith({}, sourceValue, merge);
            }
        }
        else if (targetValue === undefined || sourceValue !== undefined) {
            target[key] = sourceValue;
        }
    }
    return target;
}

export { mergeWith };
