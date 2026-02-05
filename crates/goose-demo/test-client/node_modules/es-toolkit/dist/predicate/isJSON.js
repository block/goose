'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isJSON(value) {
    if (typeof value !== 'string') {
        return false;
    }
    try {
        JSON.parse(value);
        return true;
    }
    catch {
        return false;
    }
}

exports.isJSON = isJSON;
