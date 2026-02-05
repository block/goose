'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function omitBy(obj, shouldOmit) {
    const result = {};
    const keys = Object.keys(obj);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const value = obj[key];
        if (!shouldOmit(value, key)) {
            result[key] = value;
        }
    }
    return result;
}

exports.omitBy = omitBy;
