'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const assignValue = require('../_internal/assignValue.js');

function zipObject(keys = [], values = []) {
    const result = {};
    for (let i = 0; i < keys.length; i++) {
        assignValue.assignValue(result, keys[i], values[i]);
    }
    return result;
}

exports.zipObject = zipObject;
