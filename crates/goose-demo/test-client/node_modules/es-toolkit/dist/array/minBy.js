'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function minBy(items, getValue) {
    if (items.length === 0) {
        return undefined;
    }
    let minElement = items[0];
    let min = getValue(minElement, 0, items);
    for (let i = 1; i < items.length; i++) {
        const element = items[i];
        const value = getValue(element, i, items);
        if (value < min) {
            min = value;
            minElement = element;
        }
    }
    return minElement;
}

exports.minBy = minBy;
