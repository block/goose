'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function endsWith(str, target, position) {
    if (str == null || target == null) {
        return false;
    }
    if (position == null) {
        position = str.length;
    }
    return str.endsWith(target, position);
}

exports.endsWith = endsWith;
