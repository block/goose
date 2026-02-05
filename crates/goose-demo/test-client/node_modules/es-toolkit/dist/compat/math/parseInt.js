'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function parseInt(string, radix = 0, guard) {
    if (guard) {
        radix = 0;
    }
    return Number.parseInt(string, radix);
}

exports.parseInt = parseInt;
