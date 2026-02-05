'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function getPriority(a) {
    if (typeof a === 'symbol') {
        return 1;
    }
    if (a === null) {
        return 2;
    }
    if (a === undefined) {
        return 3;
    }
    if (a !== a) {
        return 4;
    }
    return 0;
}
const compareValues = (a, b, order) => {
    if (a !== b) {
        const aPriority = getPriority(a);
        const bPriority = getPriority(b);
        if (aPriority === bPriority && aPriority === 0) {
            if (a < b) {
                return order === 'desc' ? 1 : -1;
            }
            if (a > b) {
                return order === 'desc' ? -1 : 1;
            }
        }
        return order === 'desc' ? bPriority - aPriority : aPriority - bPriority;
    }
    return 0;
};

exports.compareValues = compareValues;
