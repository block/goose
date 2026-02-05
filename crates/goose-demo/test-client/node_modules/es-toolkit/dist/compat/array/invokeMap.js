'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isFunction = require('../../predicate/isFunction.js');
const isNil = require('../../predicate/isNil.js');
const get = require('../object/get.js');
const isArrayLike = require('../predicate/isArrayLike.js');

function invokeMap(collection, path, ...args) {
    if (isNil.isNil(collection)) {
        return [];
    }
    const values = isArrayLike.isArrayLike(collection) ? Array.from(collection) : Object.values(collection);
    const result = [];
    for (let i = 0; i < values.length; i++) {
        const value = values[i];
        if (isFunction.isFunction(path)) {
            result.push(path.apply(value, args));
            continue;
        }
        const method = get.get(value, path);
        let thisContext = value;
        if (Array.isArray(path)) {
            const pathExceptLast = path.slice(0, -1);
            if (pathExceptLast.length > 0) {
                thisContext = get.get(value, pathExceptLast);
            }
        }
        else if (typeof path === 'string' && path.includes('.')) {
            const parts = path.split('.');
            const pathExceptLast = parts.slice(0, -1).join('.');
            thisContext = get.get(value, pathExceptLast);
        }
        result.push(method == null ? undefined : method.apply(thisContext, args));
    }
    return result;
}

exports.invokeMap = invokeMap;
