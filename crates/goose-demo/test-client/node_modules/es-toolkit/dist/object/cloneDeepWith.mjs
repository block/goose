import { getSymbols } from '../compat/_internal/getSymbols.mjs';
import { getTag } from '../compat/_internal/getTag.mjs';
import { uint32ArrayTag, uint16ArrayTag, uint8ClampedArrayTag, uint8ArrayTag, symbolTag, stringTag, setTag, regexpTag, objectTag, numberTag, mapTag, int32ArrayTag, int16ArrayTag, int8ArrayTag, float64ArrayTag, float32ArrayTag, dateTag, booleanTag, dataViewTag, arrayBufferTag, arrayTag, argumentsTag } from '../compat/_internal/tags.mjs';
import { isPrimitive } from '../predicate/isPrimitive.mjs';
import { isTypedArray } from '../predicate/isTypedArray.mjs';

function cloneDeepWith(obj, cloneValue) {
    return cloneDeepWithImpl(obj, undefined, obj, new Map(), cloneValue);
}
function cloneDeepWithImpl(valueToClone, keyToClone, objectToClone, stack = new Map(), cloneValue = undefined) {
    const cloned = cloneValue?.(valueToClone, keyToClone, objectToClone, stack);
    if (cloned !== undefined) {
        return cloned;
    }
    if (isPrimitive(valueToClone)) {
        return valueToClone;
    }
    if (stack.has(valueToClone)) {
        return stack.get(valueToClone);
    }
    if (Array.isArray(valueToClone)) {
        const result = new Array(valueToClone.length);
        stack.set(valueToClone, result);
        for (let i = 0; i < valueToClone.length; i++) {
            result[i] = cloneDeepWithImpl(valueToClone[i], i, objectToClone, stack, cloneValue);
        }
        if (Object.hasOwn(valueToClone, 'index')) {
            result.index = valueToClone.index;
        }
        if (Object.hasOwn(valueToClone, 'input')) {
            result.input = valueToClone.input;
        }
        return result;
    }
    if (valueToClone instanceof Date) {
        return new Date(valueToClone.getTime());
    }
    if (valueToClone instanceof RegExp) {
        const result = new RegExp(valueToClone.source, valueToClone.flags);
        result.lastIndex = valueToClone.lastIndex;
        return result;
    }
    if (valueToClone instanceof Map) {
        const result = new Map();
        stack.set(valueToClone, result);
        for (const [key, value] of valueToClone) {
            result.set(key, cloneDeepWithImpl(value, key, objectToClone, stack, cloneValue));
        }
        return result;
    }
    if (valueToClone instanceof Set) {
        const result = new Set();
        stack.set(valueToClone, result);
        for (const value of valueToClone) {
            result.add(cloneDeepWithImpl(value, undefined, objectToClone, stack, cloneValue));
        }
        return result;
    }
    if (typeof Buffer !== 'undefined' && Buffer.isBuffer(valueToClone)) {
        return valueToClone.subarray();
    }
    if (isTypedArray(valueToClone)) {
        const result = new (Object.getPrototypeOf(valueToClone).constructor)(valueToClone.length);
        stack.set(valueToClone, result);
        for (let i = 0; i < valueToClone.length; i++) {
            result[i] = cloneDeepWithImpl(valueToClone[i], i, objectToClone, stack, cloneValue);
        }
        return result;
    }
    if (valueToClone instanceof ArrayBuffer ||
        (typeof SharedArrayBuffer !== 'undefined' && valueToClone instanceof SharedArrayBuffer)) {
        return valueToClone.slice(0);
    }
    if (valueToClone instanceof DataView) {
        const result = new DataView(valueToClone.buffer.slice(0), valueToClone.byteOffset, valueToClone.byteLength);
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (typeof File !== 'undefined' && valueToClone instanceof File) {
        const result = new File([valueToClone], valueToClone.name, {
            type: valueToClone.type,
        });
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (typeof Blob !== 'undefined' && valueToClone instanceof Blob) {
        const result = new Blob([valueToClone], { type: valueToClone.type });
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (valueToClone instanceof Error) {
        const result = new valueToClone.constructor();
        stack.set(valueToClone, result);
        result.message = valueToClone.message;
        result.name = valueToClone.name;
        result.stack = valueToClone.stack;
        result.cause = valueToClone.cause;
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (valueToClone instanceof Boolean) {
        const result = new Boolean(valueToClone.valueOf());
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (valueToClone instanceof Number) {
        const result = new Number(valueToClone.valueOf());
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (valueToClone instanceof String) {
        const result = new String(valueToClone.valueOf());
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    if (typeof valueToClone === 'object' && isCloneableObject(valueToClone)) {
        const result = Object.create(Object.getPrototypeOf(valueToClone));
        stack.set(valueToClone, result);
        copyProperties(result, valueToClone, objectToClone, stack, cloneValue);
        return result;
    }
    return valueToClone;
}
function copyProperties(target, source, objectToClone = target, stack, cloneValue) {
    const keys = [...Object.keys(source), ...getSymbols(source)];
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const descriptor = Object.getOwnPropertyDescriptor(target, key);
        if (descriptor == null || descriptor.writable) {
            target[key] = cloneDeepWithImpl(source[key], key, objectToClone, stack, cloneValue);
        }
    }
}
function isCloneableObject(object) {
    switch (getTag(object)) {
        case argumentsTag:
        case arrayTag:
        case arrayBufferTag:
        case dataViewTag:
        case booleanTag:
        case dateTag:
        case float32ArrayTag:
        case float64ArrayTag:
        case int8ArrayTag:
        case int16ArrayTag:
        case int32ArrayTag:
        case mapTag:
        case numberTag:
        case objectTag:
        case regexpTag:
        case setTag:
        case stringTag:
        case symbolTag:
        case uint8ArrayTag:
        case uint8ClampedArrayTag:
        case uint16ArrayTag:
        case uint32ArrayTag: {
            return true;
        }
        default: {
            return false;
        }
    }
}

export { cloneDeepWith, cloneDeepWithImpl, copyProperties };
