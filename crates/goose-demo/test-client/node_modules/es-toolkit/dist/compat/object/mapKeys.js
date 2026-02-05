'use strict';

Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });

const identity = require('../../function/identity.js');
const mapKeys$1 = require('../../object/mapKeys.js');
const iteratee = require('../util/iteratee.js');

function mapKeys(object, getNewKey = identity.identity) {
    if (object == null) {
        return {};
    }
    return mapKeys$1.mapKeys(object, iteratee.iteratee(getNewKey));
}

exports.mapKeys = mapKeys;
