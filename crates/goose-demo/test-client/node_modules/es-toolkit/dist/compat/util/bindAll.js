'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const isFunction = require('../../predicate/isFunction.js');
const isArray = require('../predicate/isArray.js');
const isObject = require('../predicate/isObject.js');
const toString = require('./toString.js');

function bindAll(object, ...methodNames) {
    if (object == null) {
        return object;
    }
    if (!isObject.isObject(object)) {
        return object;
    }
    if (isArray.isArray(object) && methodNames.length === 0) {
        return object;
    }
    const methods = [];
    for (let i = 0; i < methodNames.length; i++) {
        const name = methodNames[i];
        if (isArray.isArray(name)) {
            methods.push(...name);
        }
        else if (name && typeof name === 'object' && 'length' in name) {
            methods.push(...Array.from(name));
        }
        else {
            methods.push(name);
        }
    }
    if (methods.length === 0) {
        return object;
    }
    for (let i = 0; i < methods.length; i++) {
        const key = methods[i];
        const stringKey = toString.toString(key);
        const func = object[stringKey];
        if (isFunction.isFunction(func)) {
            object[stringKey] = func.bind(object);
        }
    }
    return object;
}

exports.bindAll = bindAll;
