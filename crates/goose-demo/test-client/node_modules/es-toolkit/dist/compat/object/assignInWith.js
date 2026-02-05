'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keysIn = require('./keysIn.js');
const isEqualsSameValueZero = require('../../_internal/isEqualsSameValueZero.js');

function assignInWith(object, ...sources) {
    let getValueToAssign = sources[sources.length - 1];
    if (typeof getValueToAssign === 'function') {
        sources.pop();
    }
    else {
        getValueToAssign = undefined;
    }
    for (let i = 0; i < sources.length; i++) {
        assignInWithImpl(object, sources[i], getValueToAssign);
    }
    return object;
}
function assignInWithImpl(object, source, getValueToAssign) {
    const keys = keysIn.keysIn(source);
    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];
        const objValue = object[key];
        const srcValue = source[key];
        const newValue = getValueToAssign?.(objValue, srcValue, key, object, source) ?? srcValue;
        if (!(key in object) || !isEqualsSameValueZero.isEqualsSameValueZero(objValue, newValue)) {
            object[key] = newValue;
        }
    }
}

exports.assignInWith = assignInWith;
