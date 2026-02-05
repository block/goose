'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isEqualsSameValueZero = require('../_internal/isEqualsSameValueZero.js');

function hasValue(map, searchElement) {
    for (const value of map.values()) {
        if (isEqualsSameValueZero.isEqualsSameValueZero(value, searchElement)) {
            return true;
        }
    }
    return false;
}

exports.hasValue = hasValue;
