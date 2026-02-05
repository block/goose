'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function differenceWith(firstArr, secondArr, areItemsEqual) {
    return firstArr.filter(firstItem => {
        return secondArr.every(secondItem => {
            return !areItemsEqual(firstItem, secondItem);
        });
    });
}

exports.differenceWith = differenceWith;
