'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const differenceWith = require('./differenceWith.js');
const intersectionWith = require('./intersectionWith.js');
const unionWith = require('./unionWith.js');

function xorWith(arr1, arr2, areElementsEqual) {
    const union = unionWith.unionWith(arr1, arr2, areElementsEqual);
    const intersection = intersectionWith.intersectionWith(arr1, arr2, areElementsEqual);
    return differenceWith.differenceWith(union, intersection, areElementsEqual);
}

exports.xorWith = xorWith;
