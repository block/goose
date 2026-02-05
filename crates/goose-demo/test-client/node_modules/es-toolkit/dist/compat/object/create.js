'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keys = require('./keys.js');
const assignValue = require('../_internal/assignValue.js');
const isObject = require('../predicate/isObject.js');

function create(prototype, properties) {
    const proto = isObject.isObject(prototype) ? Object.create(prototype) : {};
    if (properties != null) {
        const propsKeys = keys.keys(properties);
        for (let i = 0; i < propsKeys.length; i++) {
            const key = propsKeys[i];
            const propsValue = properties[key];
            assignValue.assignValue(proto, key, propsValue);
        }
    }
    return proto;
}

exports.create = create;
