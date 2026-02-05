'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

function isPrototype(value) {
    const constructor = value?.constructor;
    const prototype = typeof constructor === 'function' ? constructor.prototype : Object.prototype;
    return value === prototype;
}

exports.isPrototype = isPrototype;
