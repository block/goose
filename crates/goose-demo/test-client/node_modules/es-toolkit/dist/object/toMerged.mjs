import { clone } from './clone.mjs';
import { mergeWith } from './mergeWith.mjs';
import { isPlainObject } from '../predicate/isPlainObject.mjs';

function toMerged(target, source) {
    return mergeWith(clone(target), source, function mergeRecursively(targetValue, sourceValue) {
        if (Array.isArray(sourceValue)) {
            if (Array.isArray(targetValue)) {
                return mergeWith(clone(targetValue), sourceValue, mergeRecursively);
            }
            else {
                return mergeWith([], sourceValue, mergeRecursively);
            }
        }
        else if (isPlainObject(sourceValue)) {
            if (isPlainObject(targetValue)) {
                return mergeWith(clone(targetValue), sourceValue, mergeRecursively);
            }
            else {
                return mergeWith({}, sourceValue, mergeRecursively);
            }
        }
    });
}

export { toMerged };
