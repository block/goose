'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function pick(obj, keys) {
    const result = {};
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        if (Object.hasOwn(obj, key)) {
            result[key] = obj[key];
        }
    }
    return result;
}

exports.pick = pick;
