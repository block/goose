'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const get = require('./get.js');
const has = require('./has.js');
const set = require('./set.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isNil = require('../predicate/isNil.js');

function pick(obj, ...keysArr) {
    if (isNil.isNil(obj)) {
        return {};
    }
    const result = {};
    for (let i = 0; i < keysArr.length; i++) {
        let keys = keysArr[i];
        switch (typeof keys) {
            case 'object': {
                if (!Array.isArray(keys)) {
                    if (isArrayLike.isArrayLike(keys)) {
                        keys = Array.from(keys);
                    }
                    else {
                        keys = [keys];
                    }
                }
                break;
            }
            case 'string':
            case 'symbol':
            case 'number': {
                keys = [keys];
                break;
            }
        }
        for (const key of keys) {
            const value = get.get(obj, key);
            if (value === undefined && !has.has(obj, key)) {
                continue;
            }
            if (typeof key === 'string' && Object.hasOwn(obj, key)) {
                result[key] = value;
            }
            else {
                set.set(result, key, value);
            }
        }
    }
    return result;
}

exports.pick = pick;
