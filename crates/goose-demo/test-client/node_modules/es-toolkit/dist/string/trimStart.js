'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function trimStart(str, chars) {
    if (chars === undefined) {
        return str.trimStart();
    }
    let startIndex = 0;
    switch (typeof chars) {
        case 'string': {
            while (startIndex < str.length && str[startIndex] === chars) {
                startIndex++;
            }
            break;
        }
        case 'object': {
            while (startIndex < str.length && chars.includes(str[startIndex])) {
                startIndex++;
            }
        }
    }
    return str.substring(startIndex);
}

exports.trimStart = trimStart;
