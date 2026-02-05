import { isObject } from './isObject.mjs';
import { isPrimitive } from '../../predicate/isPrimitive.mjs';
import { isEqualsSameValueZero } from '../../_internal/isEqualsSameValueZero.mjs';

function isMatchWith(target, source, compare) {
    if (typeof compare !== 'function') {
        return isMatchWith(target, source, () => undefined);
    }
    return isMatchWithInternal(target, source, function doesMatch(objValue, srcValue, key, object, source, stack) {
        const isEqual = compare(objValue, srcValue, key, object, source, stack);
        if (isEqual !== undefined) {
            return Boolean(isEqual);
        }
        return isMatchWithInternal(objValue, srcValue, doesMatch, stack);
    }, new Map());
}
function isMatchWithInternal(target, source, compare, stack) {
    if (source === target) {
        return true;
    }
    switch (typeof source) {
        case 'object': {
            return isObjectMatch(target, source, compare, stack);
        }
        case 'function': {
            const sourceKeys = Object.keys(source);
            if (sourceKeys.length > 0) {
                return isMatchWithInternal(target, { ...source }, compare, stack);
            }
            return isEqualsSameValueZero(target, source);
        }
        default: {
            if (!isObject(target)) {
                return isEqualsSameValueZero(target, source);
            }
            if (typeof source === 'string') {
                return source === '';
            }
            return true;
        }
    }
}
function isObjectMatch(target, source, compare, stack) {
    if (source == null) {
        return true;
    }
    if (Array.isArray(source)) {
        return isArrayMatch(target, source, compare, stack);
    }
    if (source instanceof Map) {
        return isMapMatch(target, source, compare, stack);
    }
    if (source instanceof Set) {
        return isSetMatch(target, source, compare, stack);
    }
    const keys = Object.keys(source);
    if (target == null || isPrimitive(target)) {
        return keys.length === 0;
    }
    if (keys.length === 0) {
        return true;
    }
    if (stack?.has(source)) {
        return stack.get(source) === target;
    }
    stack?.set(source, target);
    try {
        for (let i = 0; i < keys.length; i++) {
            const key = keys[i];
            if (!isPrimitive(target) && !(key in target)) {
                return false;
            }
            if (source[key] === undefined && target[key] !== undefined) {
                return false;
            }
            if (source[key] === null && target[key] !== null) {
                return false;
            }
            const isEqual = compare(target[key], source[key], key, target, source, stack);
            if (!isEqual) {
                return false;
            }
        }
        return true;
    }
    finally {
        stack?.delete(source);
    }
}
function isMapMatch(target, source, compare, stack) {
    if (source.size === 0) {
        return true;
    }
    if (!(target instanceof Map)) {
        return false;
    }
    for (const [key, sourceValue] of source.entries()) {
        const targetValue = target.get(key);
        const isEqual = compare(targetValue, sourceValue, key, target, source, stack);
        if (isEqual === false) {
            return false;
        }
    }
    return true;
}
function isArrayMatch(target, source, compare, stack) {
    if (source.length === 0) {
        return true;
    }
    if (!Array.isArray(target)) {
        return false;
    }
    const countedIndex = new Set();
    for (let i = 0; i < source.length; i++) {
        const sourceItem = source[i];
        let found = false;
        for (let j = 0; j < target.length; j++) {
            if (countedIndex.has(j)) {
                continue;
            }
            const targetItem = target[j];
            let matches = false;
            const isEqual = compare(targetItem, sourceItem, i, target, source, stack);
            if (isEqual) {
                matches = true;
            }
            if (matches) {
                countedIndex.add(j);
                found = true;
                break;
            }
        }
        if (!found) {
            return false;
        }
    }
    return true;
}
function isSetMatch(target, source, compare, stack) {
    if (source.size === 0) {
        return true;
    }
    if (!(target instanceof Set)) {
        return false;
    }
    return isArrayMatch([...target], [...source], compare, stack);
}

export { isMatchWith, isSetMatch };
