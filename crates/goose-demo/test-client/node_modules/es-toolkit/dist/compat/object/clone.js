'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isPrimitive = require('../../predicate/isPrimitive.js');
const getTag = require('../_internal/getTag.js');
const tags = require('../_internal/tags.js');
const isArray = require('../predicate/isArray.js');
const isTypedArray = require('../predicate/isTypedArray.js');

function clone(obj) {
    if (isPrimitive.isPrimitive(obj)) {
        return obj;
    }
    const tag = getTag.getTag(obj);
    if (!isCloneableObject(obj)) {
        return {};
    }
    if (isArray.isArray(obj)) {
        const result = Array.from(obj);
        if (obj.length > 0 && typeof obj[0] === 'string' && Object.hasOwn(obj, 'index')) {
            result.index = obj.index;
            result.input = obj.input;
        }
        return result;
    }
    if (isTypedArray.isTypedArray(obj)) {
        const typedArray = obj;
        const Ctor = typedArray.constructor;
        return new Ctor(typedArray.buffer, typedArray.byteOffset, typedArray.length);
    }
    if (tag === tags.arrayBufferTag) {
        return new ArrayBuffer(obj.byteLength);
    }
    if (tag === tags.dataViewTag) {
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
    if (tag === tags.booleanTag || tag === tags.numberTag || tag === tags.stringTag) {
        const Ctor = obj.constructor;
        const clone = new Ctor(obj.valueOf());
        if (tag === tags.stringTag) {
            cloneStringObjectProperties(clone, obj);
        }
        else {
            copyOwnProperties(clone, obj);
        }
        return clone;
    }
    if (tag === tags.dateTag) {
        return new Date(Number(obj));
    }
    if (tag === tags.regexpTag) {
        const regExp = obj;
        const clone = new RegExp(regExp.source, regExp.flags);
        clone.lastIndex = regExp.lastIndex;
        return clone;
    }
    if (tag === tags.symbolTag) {
        return Object(Symbol.prototype.valueOf.call(obj));
    }
    if (tag === tags.mapTag) {
        const map = obj;
        const result = new Map();
        map.forEach((obj, key) => {
            result.set(key, obj);
        });
        return result;
    }
    if (tag === tags.setTag) {
        const set = obj;
        const result = new Set();
        set.forEach(obj => {
            result.add(obj);
        });
        return result;
    }
    if (tag === tags.argumentsTag) {
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
    switch (getTag.getTag(object)) {
        case tags.argumentsTag:
        case tags.arrayTag:
        case tags.arrayBufferTag:
        case tags.dataViewTag:
        case tags.booleanTag:
        case tags.dateTag:
        case tags.float32ArrayTag:
        case tags.float64ArrayTag:
        case tags.int8ArrayTag:
        case tags.int16ArrayTag:
        case tags.int32ArrayTag:
        case tags.mapTag:
        case tags.numberTag:
        case tags.objectTag:
        case tags.regexpTag:
        case tags.setTag:
        case tags.stringTag:
        case tags.symbolTag:
        case tags.uint8ArrayTag:
        case tags.uint8ClampedArrayTag:
        case tags.uint16ArrayTag:
        case tags.uint32ArrayTag: {
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

exports.clone = clone;
