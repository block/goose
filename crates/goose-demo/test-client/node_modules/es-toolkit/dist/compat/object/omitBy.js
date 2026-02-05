'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keysIn = require('./keysIn.js');
const range = require('../../math/range.js');
const getSymbolsIn = require('../_internal/getSymbolsIn.js');
const identity = require('../function/identity.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isSymbol = require('../predicate/isSymbol.js');
const iteratee = require('../util/iteratee.js');

function omitBy(object, shouldOmit) {
    if (object == null) {
        return {};
    }
    const result = {};
    const predicate = iteratee.iteratee(shouldOmit ?? identity.identity);
    const keys = isArrayLike.isArrayLike(object)
        ? range.range(0, object.length)
        : [...keysIn.keysIn(object), ...getSymbolsIn.getSymbolsIn(object)];
    for (let i = 0; i < keys.length; i++) {
        const key = (isSymbol.isSymbol(keys[i]) ? keys[i] : keys[i].toString());
        const value = object[key];
        if (!predicate(value, key, object)) {
            result[key] = value;
        }
    }
    return result;
}

exports.omitBy = omitBy;
