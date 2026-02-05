'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function capitalize(str) {
    return (str.charAt(0).toUpperCase() + str.slice(1).toLowerCase());
}

exports.capitalize = capitalize;
