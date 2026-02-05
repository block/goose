'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function difference(firstArr, secondArr) {
    const secondSet = new Set(secondArr);
    return firstArr.filter(item => !secondSet.has(item));
}

exports.difference = difference;
