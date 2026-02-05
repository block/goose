'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const toPath = require('./toPath.js');
const toKey = require('../_internal/toKey.js');
const last = require('../array/last.js');
const get = require('../object/get.js');

function invoke(object, path, ...args) {
    args = args.flat(1);
    if (object == null) {
        return;
    }
    switch (typeof path) {
        case 'string': {
            if (typeof object === 'object' && Object.hasOwn(object, path)) {
                return invokeImpl(object, [path], args);
            }
            return invokeImpl(object, toPath.toPath(path), args);
        }
        case 'number':
        case 'symbol': {
            return invokeImpl(object, [path], args);
        }
        default: {
            if (Array.isArray(path)) {
                return invokeImpl(object, path, args);
            }
            else {
                return invokeImpl(object, [path], args);
            }
        }
    }
}
function invokeImpl(object, path, args) {
    const parent = get.get(object, path.slice(0, -1), object);
    if (parent == null) {
        return undefined;
    }
    let lastKey = last.last(path);
    const lastValue = lastKey?.valueOf();
    if (typeof lastValue === 'number') {
        lastKey = toKey.toKey(lastValue);
    }
    else {
        lastKey = String(lastKey);
    }
    const func = get.get(parent, lastKey);
    return func?.apply(parent, args);
}

exports.invoke = invoke;
