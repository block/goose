'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const updateWith = require('./updateWith.js');

function setWith(obj, path, value, customizer) {
    let customizerFn;
    if (typeof customizer === 'function') {
        customizerFn = customizer;
    }
    else {
        customizerFn = () => undefined;
    }
    return updateWith.updateWith(obj, path, () => value, customizerFn);
}

exports.setWith = setWith;
