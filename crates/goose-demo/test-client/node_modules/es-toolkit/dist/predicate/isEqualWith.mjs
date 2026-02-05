import { isPlainObject } from './isPlainObject.mjs';
import { getSymbols } from '../compat/_internal/getSymbols.mjs';
import { getTag } from '../compat/_internal/getTag.mjs';
import { functionTag, regexpTag, symbolTag, dateTag, booleanTag, numberTag, stringTag, objectTag, errorTag, dataViewTag, arrayBufferTag, float64ArrayTag, float32ArrayTag, bigInt64ArrayTag, int32ArrayTag, int16ArrayTag, int8ArrayTag, bigUint64ArrayTag, uint32ArrayTag, uint16ArrayTag, uint8ClampedArrayTag, uint8ArrayTag, arrayTag, setTag, mapTag, argumentsTag } from '../compat/_internal/tags.mjs';
import { isEqualsSameValueZero } from '../_internal/isEqualsSameValueZero.mjs';

function isEqualWith(a, b, areValuesEqual) {
    return isEqualWithImpl(a, b, undefined, undefined, undefined, undefined, areValuesEqual);
}
function isEqualWithImpl(a, b, property, aParent, bParent, stack, areValuesEqual) {
    const result = areValuesEqual(a, b, property, aParent, bParent, stack);
    if (result !== undefined) {
        return result;
    }
    if (typeof a === typeof b) {
        switch (typeof a) {
            case 'bigint':
            case 'string':
            case 'boolean':
            case 'symbol':
            case 'undefined': {
                return a === b;
            }
            case 'number': {
                return a === b || Object.is(a, b);
            }
            case 'function': {
                return a === b;
            }
            case 'object': {
                return areObjectsEqual(a, b, stack, areValuesEqual);
            }
        }
    }
    return areObjectsEqual(a, b, stack, areValuesEqual);
}
function areObjectsEqual(a, b, stack, areValuesEqual) {
    if (Object.is(a, b)) {
        return true;
    }
    let aTag = getTag(a);
    let bTag = getTag(b);
    if (aTag === argumentsTag) {
        aTag = objectTag;
    }
    if (bTag === argumentsTag) {
        bTag = objectTag;
    }
    if (aTag !== bTag) {
        return false;
    }
    switch (aTag) {
        case stringTag:
            return a.toString() === b.toString();
        case numberTag: {
            const x = a.valueOf();
            const y = b.valueOf();
            return isEqualsSameValueZero(x, y);
        }
        case booleanTag:
        case dateTag:
        case symbolTag:
            return Object.is(a.valueOf(), b.valueOf());
        case regexpTag: {
            return a.source === b.source && a.flags === b.flags;
        }
        case functionTag: {
            return a === b;
        }
    }
    stack = stack ?? new Map();
    const aStack = stack.get(a);
    const bStack = stack.get(b);
    if (aStack != null && bStack != null) {
        return aStack === b;
    }
    stack.set(a, b);
    stack.set(b, a);
    try {
        switch (aTag) {
            case mapTag: {
                if (a.size !== b.size) {
                    return false;
                }
                for (const [key, value] of a.entries()) {
                    if (!b.has(key) || !isEqualWithImpl(value, b.get(key), key, a, b, stack, areValuesEqual)) {
                        return false;
                    }
                }
                return true;
            }
            case setTag: {
                if (a.size !== b.size) {
                    return false;
                }
                const aValues = Array.from(a.values());
                const bValues = Array.from(b.values());
                for (let i = 0; i < aValues.length; i++) {
                    const aValue = aValues[i];
                    const index = bValues.findIndex(bValue => {
                        return isEqualWithImpl(aValue, bValue, undefined, a, b, stack, areValuesEqual);
                    });
                    if (index === -1) {
                        return false;
                    }
                    bValues.splice(index, 1);
                }
                return true;
            }
            case arrayTag:
            case uint8ArrayTag:
            case uint8ClampedArrayTag:
            case uint16ArrayTag:
            case uint32ArrayTag:
            case bigUint64ArrayTag:
            case int8ArrayTag:
            case int16ArrayTag:
            case int32ArrayTag:
            case bigInt64ArrayTag:
            case float32ArrayTag:
            case float64ArrayTag: {
                if (typeof Buffer !== 'undefined' && Buffer.isBuffer(a) !== Buffer.isBuffer(b)) {
                    return false;
                }
                if (a.length !== b.length) {
                    return false;
                }
                for (let i = 0; i < a.length; i++) {
                    if (!isEqualWithImpl(a[i], b[i], i, a, b, stack, areValuesEqual)) {
                        return false;
                    }
                }
                return true;
            }
            case arrayBufferTag: {
                if (a.byteLength !== b.byteLength) {
                    return false;
                }
                return areObjectsEqual(new Uint8Array(a), new Uint8Array(b), stack, areValuesEqual);
            }
            case dataViewTag: {
                if (a.byteLength !== b.byteLength || a.byteOffset !== b.byteOffset) {
                    return false;
                }
                return areObjectsEqual(new Uint8Array(a), new Uint8Array(b), stack, areValuesEqual);
            }
            case errorTag: {
                return a.name === b.name && a.message === b.message;
            }
            case objectTag: {
                const areEqualInstances = areObjectsEqual(a.constructor, b.constructor, stack, areValuesEqual) ||
                    (isPlainObject(a) && isPlainObject(b));
                if (!areEqualInstances) {
                    return false;
                }
                const aKeys = [...Object.keys(a), ...getSymbols(a)];
                const bKeys = [...Object.keys(b), ...getSymbols(b)];
                if (aKeys.length !== bKeys.length) {
                    return false;
                }
                for (let i = 0; i < aKeys.length; i++) {
                    const propKey = aKeys[i];
                    const aProp = a[propKey];
                    if (!Object.hasOwn(b, propKey)) {
                        return false;
                    }
                    const bProp = b[propKey];
                    if (!isEqualWithImpl(aProp, bProp, propKey, a, b, stack, areValuesEqual)) {
                        return false;
                    }
                }
                return true;
            }
            default: {
                return false;
            }
        }
    }
    finally {
        stack.delete(a);
        stack.delete(b);
    }
}

export { isEqualWith };
