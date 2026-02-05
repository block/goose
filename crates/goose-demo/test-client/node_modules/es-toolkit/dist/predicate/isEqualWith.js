'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isPlainObject = require('./isPlainObject.js');
const getSymbols = require('../compat/_internal/getSymbols.js');
const getTag = require('../compat/_internal/getTag.js');
const tags = require('../compat/_internal/tags.js');
const isEqualsSameValueZero = require('../_internal/isEqualsSameValueZero.js');

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
    let aTag = getTag.getTag(a);
    let bTag = getTag.getTag(b);
    if (aTag === tags.argumentsTag) {
        aTag = tags.objectTag;
    }
    if (bTag === tags.argumentsTag) {
        bTag = tags.objectTag;
    }
    if (aTag !== bTag) {
        return false;
    }
    switch (aTag) {
        case tags.stringTag:
            return a.toString() === b.toString();
        case tags.numberTag: {
            const x = a.valueOf();
            const y = b.valueOf();
            return isEqualsSameValueZero.isEqualsSameValueZero(x, y);
        }
        case tags.booleanTag:
        case tags.dateTag:
        case tags.symbolTag:
            return Object.is(a.valueOf(), b.valueOf());
        case tags.regexpTag: {
            return a.source === b.source && a.flags === b.flags;
        }
        case tags.functionTag: {
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
            case tags.mapTag: {
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
            case tags.setTag: {
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
            case tags.arrayTag:
            case tags.uint8ArrayTag:
            case tags.uint8ClampedArrayTag:
            case tags.uint16ArrayTag:
            case tags.uint32ArrayTag:
            case tags.bigUint64ArrayTag:
            case tags.int8ArrayTag:
            case tags.int16ArrayTag:
            case tags.int32ArrayTag:
            case tags.bigInt64ArrayTag:
            case tags.float32ArrayTag:
            case tags.float64ArrayTag: {
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
            case tags.arrayBufferTag: {
                if (a.byteLength !== b.byteLength) {
                    return false;
                }
                return areObjectsEqual(new Uint8Array(a), new Uint8Array(b), stack, areValuesEqual);
            }
            case tags.dataViewTag: {
                if (a.byteLength !== b.byteLength || a.byteOffset !== b.byteOffset) {
                    return false;
                }
                return areObjectsEqual(new Uint8Array(a), new Uint8Array(b), stack, areValuesEqual);
            }
            case tags.errorTag: {
                return a.name === b.name && a.message === b.message;
            }
            case tags.objectTag: {
                const areEqualInstances = areObjectsEqual(a.constructor, b.constructor, stack, areValuesEqual) ||
                    (isPlainObject.isPlainObject(a) && isPlainObject.isPlainObject(b));
                if (!areEqualInstances) {
                    return false;
                }
                const aKeys = [...Object.keys(a), ...getSymbols.getSymbols(a)];
                const bKeys = [...Object.keys(b), ...getSymbols.getSymbols(b)];
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

exports.isEqualWith = isEqualWith;
