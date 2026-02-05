import { isPrimitive } from '../../predicate/isPrimitive.mjs';
import { getTag } from '../_internal/getTag.mjs';
import { arrayBufferTag, dataViewTag, booleanTag, numberTag, stringTag, dateTag, regexpTag, symbolTag, mapTag, setTag, argumentsTag, uint32ArrayTag, uint16ArrayTag, uint8ClampedArrayTag, uint8ArrayTag, objectTag, int32ArrayTag, int16ArrayTag, int8ArrayTag, float64ArrayTag, float32ArrayTag, arrayTag } from '../_internal/tags.mjs';
import { isArray } from '../predicate/isArray.mjs';
import { isTypedArray } from '../predicate/isTypedArray.mjs';

function clone(obj) {
    if (isPrimitive(obj)) {
        return obj;
    }
    const tag = getTag(obj);
    if (!isCloneableObject(obj)) {
        return {};
    }
    if (isArray(obj)) {
        const result = Array.from(obj);
        if (obj.length > 0 && typeof obj[0] === 'string' && Object.hasOwn(obj, 'index')) {
            result.index = obj.index;
            result.input = obj.input;
        }
        return result;
    }
    if (isTypedArray(obj)) {
        const typedArray = obj;
        const Ctor = typedArray.constructor;
        return new Ctor(typedArray.buffer, typedArray.byteOffset, typedArray.length);
    }
    if (tag === arrayBufferTag) {
        return new ArrayBuffer(obj.byteLength);
    }
    if (tag === dataViewTag) {
        const dataView = obj;
        const buffer = dataView.buffer;
        const byteOffset = dataView.byteOffset;
        const byteLength = dataView.byteLength;
        const clonedBuffer = new ArrayBuffer(byteLength);
        const srcView = new Uint8Array(buffer, byteOffset, byteLength);
        const destView = new Uint8Array(clonedBuffer);
        destView.set(srcView);
        return new DataView(clonedBuffer);
    }
    if (tag === booleanTag || tag === numberTag || tag === stringTag) {
        const Ctor = obj.constructor;
        const clone = new Ctor(obj.valueOf());
        if (tag === stringTag) {
            cloneStringObjectProperties(clone, obj);
        }
        else {
            copyOwnProperties(clone, obj);
        }
        return clone;
    }
    if (tag === dateTag) {
        return new Date(Number(obj));
    }
    if (tag === regexpTag) {
        const regExp = obj;
        const clone = new RegExp(regExp.source, regExp.flags);
        clone.lastIndex = regExp.lastIndex;
        return clone;
    }
    if (tag === symbolTag) {
        return Object(Symbol.prototype.valueOf.call(obj));
    }
    if (tag === mapTag) {
        const map = obj;
        const result = new Map();
        map.forEach((obj, key) => {
            result.set(key, obj);
        });
        return result;
    }
    if (tag === setTag) {
        const set = obj;
        const result = new Set();
        set.forEach(obj => {
            result.add(obj);
        });
        return result;
    }
    if (tag === argumentsTag) {
        const args = obj;
        const result = {};
        copyOwnProperties(result, args);
        result.length = args.length;
        result[Symbol.iterator] = args[Symbol.iterator];
        return result;
    }
    const result = {};
    copyPrototype(result, obj);
    copyOwnProperties(result, obj);
    copySymbolProperties(result, obj);
    return result;
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
function copyOwnProperties(target, source) {
    for (const key in source) {
        if (Object.hasOwn(source, key)) {
            target[key] = source[key];
        }
    }
}
function copySymbolProperties(target, source) {
    const symbols = Object.getOwnPropertySymbols(source);
    for (let i = 0; i < symbols.length; i++) {
        const symbol = symbols[i];
        if (Object.prototype.propertyIsEnumerable.call(source, symbol)) {
            target[symbol] = source[symbol];
        }
    }
}
function cloneStringObjectProperties(target, source) {
    const stringLength = source.valueOf().length;
    for (const key in source) {
        if (Object.hasOwn(source, key) && (Number.isNaN(Number(key)) || Number(key) >= stringLength)) {
            target[key] = source[key];
        }
    }
}
function copyPrototype(target, source) {
    const proto = Object.getPrototypeOf(source);
    if (proto !== null) {
        const Ctor = source.constructor;
        if (typeof Ctor === 'function') {
            Object.setPrototypeOf(target, proto);
        }
    }
}

export { clone };
