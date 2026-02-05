'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

const assignValue = (object, key, value) => {
    const objValue = object[key];
    if (!(Object.hasOwn(object, key) && isEqualsSameValueZero.isEqualsSameValueZero(objValue, value)) || (value === undefined && !(key in object))) {
        object[key] = value;
    }
};

exports.assignValue = assignValue;
