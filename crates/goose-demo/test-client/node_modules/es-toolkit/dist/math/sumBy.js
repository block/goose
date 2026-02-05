'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function sumBy(items, getValue) {
    let result = 0;
    for (let i = 0; i < items.length; i++) {
        result += getValue(items[i], i);
    }
    return result;
}

exports.sumBy = sumBy;
