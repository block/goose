'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isDeepKey(key) {
    switch (typeof key) {
        case 'number':
        case 'symbol': {
            return false;
        }
        case 'string': {
            return key.includes('.') || key.includes('[') || key.includes(']');
        }
    }
}

exports.isDeepKey = isDeepKey;
