'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const cloneDeepWith = require('./cloneDeepWith.js');
const keysIn = require('./keysIn.js');
const unset = require('./unset.js');
const getSymbolsIn = require('../_internal/getSymbolsIn.js');
const isDeepKey = require('../_internal/isDeepKey.js');
const flatten = require('../array/flatten.js');
const isPlainObject = require('../predicate/isPlainObject.js');

function omit(obj, ...keysArr) {
    if (obj == null) {
        return {};
    }
    keysArr = flatten.flatten(keysArr);
    const result = cloneInOmit(obj, keysArr);
    for (let i = 0; i < keysArr.length; i++) {
        let keys = keysArr[i];
        switch (typeof keys) {
            case 'object': {
                if (!Array.isArray(keys)) {
                    keys = Array.from(keys);
                }
                for (let j = 0; j < keys.length; j++) {
                    const key = keys[j];
                    unset.unset(result, key);
                }
                break;
            }
            case 'string':
            case 'symbol':
            case 'number': {
                unset.unset(result, keys);
                break;
            }
        }
    }
    return result;
}
function cloneInOmit(obj, keys) {
    const hasDeepKey = keys.some(key => Array.isArray(key) || isDeepKey.isDeepKey(key));
    if (hasDeepKey) {
        return deepCloneInOmit(obj);
    }
    return shallowCloneInOmit(obj);
}
function shallowCloneInOmit(obj) {
    const result = {};
    const keysToCopy = [...keysIn.keysIn(obj), ...getSymbolsIn.getSymbolsIn(obj)];
    for (let i = 0; i < keysToCopy.length; i++) {
        const key = keysToCopy[i];
        result[key] = obj[key];
    }
    return result;
}
function deepCloneInOmit(obj) {
    const result = {};
    const keysToCopy = [...keysIn.keysIn(obj), ...getSymbolsIn.getSymbolsIn(obj)];
    for (let i = 0; i < keysToCopy.length; i++) {
        const key = keysToCopy[i];
        result[key] = cloneDeepWith.cloneDeepWith(obj[key], valueToClone => {
            if (isPlainObject.isPlainObject(valueToClone)) {
                return undefined;
            }
            return valueToClone;
        });
    }
    return result;
}

exports.omit = omit;
