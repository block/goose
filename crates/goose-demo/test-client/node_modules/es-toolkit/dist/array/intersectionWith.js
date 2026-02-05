'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function intersectionWith(firstArr, secondArr, areItemsEqual) {
    return firstArr.filter(firstItem => {
        return secondArr.some(secondItem => {
            return areItemsEqual(firstItem, secondItem);
        });
    });
}

exports.intersectionWith = intersectionWith;
