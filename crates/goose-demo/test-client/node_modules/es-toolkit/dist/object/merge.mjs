import { isUnsafeProperty } from '../_internal/isUnsafeProperty.mjs';
import { isPlainObject } from '../predicate/isPlainObject.mjs';

function merge(target, source) {
    const sourceKeys = Object.keys(source);
    for (let i = 0; i < sourceKeys.length; i++) {
        const key = sourceKeys[i];
        if (isUnsafeProperty(key)) {
            continue;
        }
        const sourceValue = source[key];
        const targetValue = target[key];
        if (isMergeableValue(sourceValue) && isMergeableValue(targetValue)) {
            target[key] = merge(targetValue, sourceValue);
        }
        else if (Array.isArray(sourceValue)) {
            target[key] = merge([], sourceValue);
        }
        else if (isPlainObject(sourceValue)) {
            target[key] = merge({}, sourceValue);
        }
        else if (targetValue === undefined || sourceValue !== undefined) {
            target[key] = sourceValue;
        }
    }
    return target;
}
function isMergeableValue(value) {
    return isPlainObject(value) || Array.isArray(value);
}

export { merge };
