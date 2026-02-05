'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const last = require('./last.js');
const intersectionWith$1 = require('../../array/intersectionWith.js');
const uniq = require('./uniq.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

function intersectionWith(firstArr, ...otherArrs) {
    if (firstArr == null) {
        return [];
    }
    const _comparator = last.last(otherArrs);
    let comparator = isEqualsSameValueZero.isEqualsSameValueZero;
    let uniq$1 = uniq.uniq;
    if (typeof _comparator === 'function') {
        comparator = _comparator;
        uniq$1 = uniqPreserve0;
        otherArrs.pop();
    }
    let result = uniq$1(Array.from(firstArr));
    for (let i = 0; i < otherArrs.length; ++i) {
        const otherArr = otherArrs[i];
        if (otherArr == null) {
            return [];
        }
        result = intersectionWith$1.intersectionWith(result, Array.from(otherArr), comparator);
    }
    return result;
}
function uniqPreserve0(arr) {
    const result = [];
    const added = new Set();
    for (let i = 0; i < arr.length; i++) {
        const item = arr[i];
        if (added.has(item)) {
            continue;
        }
        result.push(item);
        added.add(item);
    }
    return result;
}

exports.intersectionWith = intersectionWith;
