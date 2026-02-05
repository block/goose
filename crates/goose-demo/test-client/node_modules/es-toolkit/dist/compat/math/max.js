'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function max(items) {
    if (!items || items.length === 0) {
        return undefined;
    }
    let maxResult = undefined;
    for (let i = 0; i < items.length; i++) {
        const current = items[i];
        if (current == null || Number.isNaN(current) || typeof current === 'symbol') {
            continue;
        }
        if (maxResult === undefined || current > maxResult) {
            maxResult = current;
        }
    }
    return maxResult;
}

exports.max = max;
