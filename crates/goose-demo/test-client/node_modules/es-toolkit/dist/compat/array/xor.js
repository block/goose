'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isArrayLikeObject = require('../predicate/isArrayLikeObject.js');
const toArray = require('../util/toArray.js');

function xor(...arrays) {
    const itemCounts = new Map();
    for (let i = 0; i < arrays.length; i++) {
        const array = arrays[i];
        if (!isArrayLikeObject.isArrayLikeObject(array)) {
            continue;
        }
        const itemSet = new Set(toArray.toArray(array));
        for (const item of itemSet) {
            if (!itemCounts.has(item)) {
                itemCounts.set(item, 1);
            }
            else {
                itemCounts.set(item, itemCounts.get(item) + 1);
            }
        }
    }
    const result = [];
    for (const [item, count] of itemCounts) {
        if (count === 1) {
            result.push(item);
        }
    }
    return result;
}

exports.xor = xor;
