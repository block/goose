'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const keysIn = require('./keysIn.js');
const range = require('../../math/range.js');
const getSymbolsIn = require('../_internal/getSymbolsIn.js');
const identity = require('../function/identity.js');
const isArrayLike = require('../predicate/isArrayLike.js');
const isSymbol = require('../predicate/isSymbol.js');
const iteratee = require('../util/iteratee.js');

function pickBy(obj, shouldPick) {
    if (obj == null) {
        return {};
    }
    const predicate = iteratee.iteratee(shouldPick ?? identity.identity);
    const result = {};
    const keys = isArrayLike.isArrayLike(obj) ? range.range(0, obj.length) : [...keysIn.keysIn(obj), ...getSymbolsIn.getSymbolsIn(obj)];
    for (let i = 0; i < keys.length; i++) {
        const key = (isSymbol.isSymbol(keys[i]) ? keys[i] : keys[i].toString());
        const value = obj[key];
        if (predicate(value, key, obj)) {
            result[key] = value;
        }
    }
    return result;
}

exports.pickBy = pickBy;
